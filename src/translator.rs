//! Declaration -> schema node translation (counterpart of `node-translator.c++`).
//!
//! `translate_node` builds the "bootstrap" node (everything except values of non-primitive type,
//! which are recorded as pending); `finish_translator` evaluates the pending values.

use crate::ast::*;
use crate::compiler::*;
use crate::error::Span;
use crate::generics::{BrandScope, BrandedDecl, ImplicitParams};
use crate::ids::{generate_group_id, generate_method_params_id};
use crate::layout::{Layout, Scope};
use crate::parser::expression_string;
use crate::schema::*;
use std::collections::BTreeMap;
use std::rc::Rc;

pub struct AuxNode {
    pub node: Node,
    pub source_info: SourceInfo,
}

pub struct Unfinished {
    pub source: Expression,
    pub ty: Type,
    pub default: Value,
}

pub struct Translator {
    pub node_idx: NodeIdx,
    pub module: usize,
    pub decl: Rc<Declaration>,
    pub wip: Node,
    pub source_info: SourceInfo,
    pub groups: Vec<AuxNode>,
    pub param_structs: Vec<AuxNode>,
    pub unfinished: Vec<Unfinished>,
    pub local_brand: Rc<BrandScope>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Target {
    File,
    Const,
    Enum,
    Enumerant,
    Struct,
    Field,
    Union,
    Group,
    Interface,
    Method,
    Param,
    Annotation,
}

fn has_target(t: &Targets, k: Target) -> bool {
    match k {
        Target::File => t.file,
        Target::Const => t.constant,
        Target::Enum => t.enumeration,
        Target::Enumerant => t.enumerant,
        Target::Struct => t.structure,
        Target::Field => t.field,
        Target::Union => t.union,
        Target::Group => t.group,
        Target::Interface => t.interface,
        Target::Method => t.method,
        Target::Param => t.param,
        Target::Annotation => t.annotation,
    }
}

impl Translator {
    pub fn bootstrap_set(&self) -> NodeSetRc {
        let mut infos = vec![self.source_info.clone()];
        for g in &self.groups {
            infos.push(g.source_info.clone());
        }
        for p in &self.param_structs {
            infos.push(p.source_info.clone());
        }
        let is_interface = matches!(self.wip.kind, NodeKind::Interface(_));
        let aux: Vec<Rc<Node>> = if is_interface {
            self.param_structs.iter().map(|g| Rc::new(g.node.clone())).collect()
        } else {
            self.groups.iter().map(|g| Rc::new(g.node.clone())).collect()
        };
        NodeSetRc { node: Rc::new(self.wip.clone()), aux, source_info: infos }
    }
}

/// Build the header of the node for `idx` and translate its declaration.
pub fn translate_node(c: &mut Compiler, idx: NodeIdx) -> Translator {
    let decl = c.decl(idx);
    let id = c.nodes[idx].id;
    let display = c.nodes[idx].display_name.clone();
    let mut prefix = 0u32;
    if let Some(dot) = display.rfind('.') {
        prefix = dot as u32 + 1;
    }
    if let Some(colon) = display.rfind(':') {
        if colon as u32 > prefix {
            prefix = colon as u32 + 1;
        }
    }
    let scope_id = c.nodes[idx].parent.map(|p| c.nodes[p].id).unwrap_or(0);
    let nested: Vec<(String, Id)> =
        c.nodes[idx].nested.iter().map(|(n, ci)| (n.clone(), c.nodes[*ci].id)).collect();
    let module = c.nodes[idx].module;
    let local_brand = BrandScope::new_root(c, module, id, decl.parameters.len(), idx);
    let wip = Node {
        id,
        display_name: display,
        display_name_prefix_length: prefix,
        scope_id,
        nested_nodes: Some(nested),
        annotations: vec![],
        parameters: vec![],
        is_generic: false,
        start_byte: 0,
        end_byte: 0,
        kind: NodeKind::File,
    };
    let mut ts = Translator {
        node_idx: idx,
        module,
        decl: decl.clone(),
        wip,
        source_info: SourceInfo::default(),
        groups: vec![],
        param_structs: vec![],
        unfinished: vec![],
        local_brand,
    };
    compile_node(c, &mut ts, &decl);
    ts
}

fn compile_node(c: &mut Compiler, ts: &mut Translator, decl: &Declaration) {
    check_duplicate_names(c, ts.module, &decl.nested, decl_kind(decl), &mut BTreeMap::new());

    if !decl.parameters.is_empty() {
        ts.wip.parameters = decl.parameters.iter().map(|p| p.name.clone()).collect();
    }
    ts.wip.is_generic = ts.local_brand.is_generic();

    let target = match &decl.body {
        DeclBody::File => Target::File,
        DeclBody::Const { ty, value } => {
            let (t, v) = compile_const(c, ts, ty, value);
            ts.wip.kind = NodeKind::Const { ty: t, value: v };
            Target::Const
        }
        DeclBody::Annotation { ty, targets } => {
            let t = compile_type(c, ts, ty, &ImplicitParams::none()).unwrap_or(Type::Void);
            ts.wip.kind = NodeKind::Annotation { ty: t, targets: *targets };
            Target::Annotation
        }
        DeclBody::Enum => {
            compile_enum(c, ts, &decl.nested);
            Target::Enum
        }
        DeclBody::Struct => {
            compile_struct(c, ts, &decl.nested);
            Target::Struct
        }
        DeclBody::Interface { superclasses } => {
            compile_interface(c, ts, superclasses, &decl.nested);
            Target::Interface
        }
        _ => panic!("This Declaration is not a node."),
    };
    ts.wip.start_byte = decl.span.start;
    ts.wip.end_byte = decl.span.end;
    ts.wip.annotations = compile_annotation_applications(c, ts, &decl.annotations, target);

    ts.source_info.id = ts.wip.id;
    ts.source_info.doc_comment = decl.doc_comment.clone();
    ts.source_info.start_byte = decl.span.start;
    ts.source_info.end_byte = decl.span.end;
}

fn expression_target_name(e: &Expression) -> &str {
    match &e.kind {
        ExprKind::AbsoluteName(n) | ExprKind::RelativeName(n) => n,
        ExprKind::Application { function, .. } => expression_target_name(function),
        ExprKind::Member { name, .. } => &name.value,
        _ => "",
    }
}

fn check_duplicate_names(
    c: &mut Compiler,
    module: usize,
    nested: &[Rc<Declaration>],
    parent_kind: DeclKind,
    names: &mut BTreeMap<String, Span>,
) {
    for decl in nested {
        let name = &decl.name;
        let text = &name.value;
        let kind = decl_kind(decl);
        if let Some(prev) = names.get(text).copied() {
            if text.is_empty() && matches!(decl.body, DeclBody::Union) {
                c.add_error(module, name.span, "An unnamed union is already defined in this scope.");
                c.add_error(module, prev, "Previously defined here.");
            } else {
                c.add_error(module, name.span, format!("'{text}' is already defined in this scope."));
                c.add_error(module, prev, format!("'{text}' previously defined here."));
            }
        } else {
            names.insert(text.clone(), name.span);
        }

        let first = text.chars().next();
        match &decl.body {
            DeclBody::Using { target } => {
                let tn = expression_target_name(target);
                if tn.chars().next().is_some_and(|ch| ch.is_ascii_lowercase()) {
                    if first.is_some_and(|ch| !ch.is_ascii_lowercase()) {
                        c.add_error(module, name.span, "Non-type names must begin with a lower-case letter.");
                    }
                } else if first.is_some_and(|ch| !ch.is_ascii_uppercase()) {
                    c.add_error(module, name.span, "Type names must begin with a capital letter.");
                }
            }
            DeclBody::Enum | DeclBody::Struct | DeclBody::Interface { .. } => {
                if first.is_some_and(|ch| !ch.is_ascii_uppercase()) {
                    c.add_error(module, name.span, "Type names must begin with a capital letter.");
                }
            }
            DeclBody::Const { .. }
            | DeclBody::Annotation { .. }
            | DeclBody::Enumerant
            | DeclBody::Method { .. }
            | DeclBody::Field { .. }
            | DeclBody::Union
            | DeclBody::Group => {
                if first.is_some_and(|ch| !ch.is_ascii_lowercase()) {
                    c.add_error(module, name.span, "Non-type names must begin with a lower-case letter.");
                }
            }
            _ => {}
        }
        if text.contains('_') {
            c.add_error(
                module,
                name.span,
                "Cap'n Proto declaration names should use camelCase and must not contain underscores. (Code generators may convert names to the appropriate style for the target language.)",
            );
        }

        match kind {
            DeclKind::Using | DeclKind::Const | DeclKind::Enum | DeclKind::Struct | DeclKind::Interface | DeclKind::Annotation => {
                if !matches!(parent_kind, DeclKind::File | DeclKind::Struct | DeclKind::Interface) {
                    c.add_error(module, decl.span, "This kind of declaration doesn't belong here.");
                }
            }
            DeclKind::Enumerant => {
                if parent_kind != DeclKind::Enum {
                    c.add_error(module, decl.span, "Enumerants can only appear in enums.");
                }
            }
            DeclKind::Method => {
                if parent_kind != DeclKind::Interface {
                    c.add_error(module, decl.span, "Methods can only appear in interfaces.");
                }
            }
            DeclKind::Field | DeclKind::Union | DeclKind::Group => {
                if !matches!(parent_kind, DeclKind::Struct | DeclKind::Union | DeclKind::Group) {
                    c.add_error(module, decl.span, "This declaration can only appear in structs.");
                }
                if text.is_empty() {
                    check_duplicate_names(c, module, &decl.nested, kind, names);
                } else {
                    check_duplicate_names(c, module, &decl.nested, kind, &mut BTreeMap::new());
                }
            }
            _ => c.add_error(module, decl.span, "This kind of declaration doesn't belong here."),
        }
    }
}

struct DupOrdinal {
    expected: u64,
    last: Option<Located<u64>>,
}

impl DupOrdinal {
    fn new() -> Self {
        DupOrdinal { expected: 0, last: None }
    }
    fn check(&mut self, c: &mut Compiler, module: usize, ord: &Located<u64>) {
        if ord.value < self.expected {
            c.add_error(module, ord.span, "Duplicate ordinal number.");
            if let Some(last) = self.last.take() {
                c.add_error(module, last.span, format!("Ordinal @{} originally used here.", last.value));
            }
        } else if ord.value > self.expected {
            c.add_error(
                module,
                ord.span,
                format!("Skipped ordinal @{}.  Ordinals must be sequential with no holes.", self.expected),
            );
            self.expected = ord.value + 1;
        } else {
            self.expected += 1;
            self.last = Some(ord.clone());
        }
    }
}

// ---------------------------------------------------------------------------------------------
// types and values

pub fn compile_type(c: &mut Compiler, ts: &Translator, source: &Expression, implicit: &ImplicitParams) -> Option<Type> {
    let decl = ts.local_brand.compile_decl_expression(c, source, ts.node_idx, implicit)?;
    decl.compile_as_type(c, ts.module)
}

fn compile_decl_expr(
    c: &mut Compiler,
    ts: &Translator,
    source: &Expression,
    implicit: &ImplicitParams,
) -> Option<BrandedDecl> {
    ts.local_brand.compile_decl_expression(c, source, ts.node_idx, implicit)
}

pub fn default_default_value(ty: &Type) -> Value {
    match ty {
        Type::Void => Value::Void,
        Type::Bool => Value::Bool(false),
        Type::Int8 => Value::Int8(0),
        Type::Int16 => Value::Int16(0),
        Type::Int32 => Value::Int32(0),
        Type::Int64 => Value::Int64(0),
        Type::UInt8 => Value::UInt8(0),
        Type::UInt16 => Value::UInt16(0),
        Type::UInt32 => Value::UInt32(0),
        Type::UInt64 => Value::UInt64(0),
        Type::Float32 => Value::Float32(0.0),
        Type::Float64 => Value::Float64(0.0),
        Type::Enum { .. } => Value::Enum(0),
        Type::Interface { .. } => Value::Interface,
        Type::Text => Value::Text(Ptr::Null),
        Type::Data => Value::Data(Ptr::Null),
        Type::Struct { .. } => Value::Struct(Ptr::Null),
        Type::List(_) => Value::List(Ptr::Null),
        Type::AnyPointer(_) => Value::AnyPointer(Ptr::Null),
    }
}

/// `compileBootstrapValue`: primitives are evaluated now, pointer values are deferred.
pub fn compile_bootstrap_value(c: &mut Compiler, ts: &mut Translator, source: &Expression, ty: &Type) -> Value {
    let default = default_default_value(ty);
    match ty {
        Type::List(_) | Type::Struct { .. } | Type::Interface { .. } | Type::AnyPointer(_) => {
            ts.unfinished.push(Unfinished { source: source.clone(), ty: ty.clone(), default });
            Value::Pending(ts.unfinished.len() - 1)
        }
        _ => crate::value::compile_value(c, ts, source, ty, true).unwrap_or(default),
    }
}

fn compile_const(c: &mut Compiler, ts: &mut Translator, ty_expr: &Expression, value: &Expression) -> (Type, Value) {
    match compile_type(c, ts, ty_expr, &ImplicitParams::none()) {
        Some(t) => {
            let v = compile_bootstrap_value(c, ts, value, &t);
            (t, v)
        }
        None => (Type::Void, Value::Void),
    }
}

pub fn compile_annotation_applications(
    c: &mut Compiler,
    ts: &mut Translator,
    annotations: &[AnnotationApplication],
    target: Target,
) -> Vec<Annotation> {
    let mut out = vec![];
    for a in annotations {
        let mut ann = Annotation { id: 0, value: Value::Void, brand: Brand::default() };
        let name = &a.name;
        if let Some(decl) = compile_decl_expr(c, ts, name, &ImplicitParams::none()) {
            match decl.kind() {
                Some(DeclKind::Annotation) => {
                    let (id, brand) = decl.id_and_brand(c);
                    ann.id = id;
                    ann.brand = brand;
                    if let Some(anode) = c.resolve_bootstrap_schema(id) {
                        if let NodeKind::Annotation { ty, targets } = &anode.kind {
                            if !has_target(targets, target) {
                                c.add_error(
                                    ts.module,
                                    name.span,
                                    format!("'{}' cannot be applied to this kind of declaration.", expression_string(name)),
                                );
                            }
                            match &a.value {
                                None => {
                                    if matches!(ty, Type::Void) {
                                        ann.value = Value::Void;
                                    } else {
                                        c.add_error(
                                            ts.module,
                                            name.span,
                                            format!("'{}' requires a value.", expression_string(name)),
                                        );
                                        ann.value = default_default_value(ty);
                                    }
                                }
                                Some(expr) => {
                                    ann.value = compile_bootstrap_value(c, ts, expr, ty);
                                }
                            }
                        }
                    }
                }
                _ => c.add_error(
                    ts.module,
                    name.span,
                    format!("'{}' is not an annotation.", expression_string(name)),
                ),
            }
        }
        out.push(ann);
    }
    out
}

// ---------------------------------------------------------------------------------------------
// enums

fn compile_enum(c: &mut Compiler, ts: &mut Translator, members: &[Rc<Declaration>]) {
    let mut enumerants: Vec<(u64, u32, Rc<Declaration>)> = vec![];
    let mut code_order = 0u32;
    for m in members {
        if matches!(m.body, DeclBody::Enumerant) {
            enumerants.push((m.ordinal().unwrap_or(0), code_order, m.clone()));
            code_order += 1;
        }
    }
    enumerants.sort_by_key(|e| e.0); // stable: equal ordinals keep declaration order
    let mut list = vec![];
    let mut infos = vec![];
    let mut dup = DupOrdinal::new();
    for (_, code_order, d) in &enumerants {
        if let DeclId::Ordinal(o) = &d.id {
            dup.check(c, ts.module, o);
        }
        infos.push(MemberSourceInfo { doc_comment: d.doc_comment.clone(), start_byte: d.span.start, end_byte: d.span.end });
        let annotations = compile_annotation_applications(c, ts, &d.annotations, Target::Enumerant);
        list.push(Enumerant { name: d.name.value.clone(), code_order: *code_order as u16, annotations });
    }
    ts.wip.kind = NodeKind::Enum(list);
    ts.source_info.members = Some(infos);
}

// ---------------------------------------------------------------------------------------------
// structs

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NodeRef {
    Top,
    Group(usize),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MKind {
    Field,
    Union,
    Group,
}

struct MemberInfo {
    parent: Option<usize>,
    code_order: u32,
    index: u32,
    child_count: u32,
    child_initialized_count: u32,
    union_discriminant_count: u32,
    is_in_union: bool,
    name: String,
    ordinal: Option<Located<u64>>,
    kind: MKind,
    is_param: bool,
    field_type: Option<Expression>,
    field_default: Option<Expression>,
    annotations: Vec<AnnotationApplication>,
    start_byte: u32,
    end_byte: u32,
    doc_comment: Option<String>,
    schema: Option<usize>,
    node: NodeRef,
    field_scope: Scope,
    union_scope: Option<usize>,
}

struct StructTranslator {
    layout: Layout,
    members: Vec<MemberInfo>,
    members_by_ordinal: Vec<(u64, usize)>,
    all_members: Vec<usize>,
    top: Node,
    top_info: SourceInfo,
    groups: Vec<AuxNode>,
    implicit: ImplicitParams,
}

impl StructTranslator {
    fn node_mut(&mut self, r: NodeRef) -> &mut Node {
        match r {
            NodeRef::Top => &mut self.top,
            NodeRef::Group(i) => &mut self.groups[i].node,
        }
    }
    fn node_ref(&self, r: NodeRef) -> &Node {
        match r {
            NodeRef::Top => &self.top,
            NodeRef::Group(i) => &self.groups[i].node,
        }
    }
    fn info_mut(&mut self, r: NodeRef) -> &mut SourceInfo {
        match r {
            NodeRef::Top => &mut self.top_info,
            NodeRef::Group(i) => &mut self.groups[i].source_info,
        }
    }

    fn struct_of(&mut self, r: NodeRef) -> &mut StructNode {
        match &mut self.node_mut(r).kind {
            NodeKind::Struct(s) => s,
            _ => panic!("not a struct node"),
        }
    }

    fn new_member(
        &mut self,
        parent: usize,
        code_order: u32,
        decl: &Declaration,
        kind: MKind,
        field_scope: Scope,
        node: NodeRef,
        is_in_union: bool,
    ) -> usize {
        let (field_type, field_default) = match &decl.body {
            DeclBody::Field { ty, default_value } => (Some(ty.clone()), default_value.clone()),
            _ => (None, None),
        };
        self.members.push(MemberInfo {
            parent: Some(parent),
            code_order,
            index: 0,
            child_count: 0,
            child_initialized_count: 0,
            union_discriminant_count: 0,
            is_in_union,
            name: decl.name.value.clone(),
            ordinal: match &decl.id {
                DeclId::Ordinal(o) => Some(o.clone()),
                _ => None,
            },
            kind,
            is_param: false,
            field_type,
            field_default,
            annotations: decl.annotations.clone(),
            start_byte: decl.span.start,
            end_byte: decl.span.end,
            doc_comment: decl.doc_comment.clone(),
            schema: None,
            node,
            field_scope,
            union_scope: None,
        });
        self.members.len() - 1
    }

    fn new_group_node(&mut self, parent_node: NodeRef, decl: &Declaration) -> NodeRef {
        let (parent_name, parent_generic) = {
            let p = self.node_ref(parent_node);
            (p.display_name.clone(), p.is_generic)
        };
        let name = &decl.name.value;
        let display = format!("{parent_name}.{name}");
        let prefix = (display.len() - name.len()) as u32;
        let node = Node {
            id: 0,
            display_name: display,
            display_name_prefix_length: prefix,
            scope_id: 0,
            nested_nodes: None,
            annotations: vec![],
            parameters: vec![],
            is_generic: parent_generic,
            // capnp 1.4.0 leaves the locations of group nodes unset.
            start_byte: 0,
            end_byte: 0,
            kind: NodeKind::Struct(StructNode { is_group: true, ..Default::default() }),
        };
        let info = SourceInfo::default();
        self.groups.push(AuxNode { node, source_info: info });
        NodeRef::Group(self.groups.len() - 1)
    }

    /// `MemberInfo::addMemberSchema` for member `m`'s node: returns (field index, member info index).
    fn add_member_schema(&mut self, m: usize) -> usize {
        let node_ref = self.members[m].node;
        let child_count = self.members[m].child_count as usize;
        let has_fields = !self.struct_of(node_ref).fields.is_empty();
        if !has_fields {
            if self.members[m].parent.is_some() {
                self.get_schema(m);
            }
            let placeholder = Field {
                name: String::new(),
                code_order: 0,
                annotations: vec![],
                discriminant_value: NO_DISCRIMINANT,
                kind: FieldKind::Group { type_id: 0 },
                ordinal: Ordinal::Implicit,
            };
            self.struct_of(node_ref).fields = vec![placeholder; child_count];
            self.info_mut(node_ref).members = Some(vec![MemberSourceInfo::default(); child_count]);
        }
        let i = self.members[m].child_initialized_count as usize;
        self.members[m].child_initialized_count += 1;
        i
    }

    /// `MemberInfo::getSchema`: returns (node ref, field index) of this member's field.
    fn get_schema(&mut self, m: usize) -> (NodeRef, usize) {
        let parent = self.members[m].parent.expect("root has no field schema");
        let pnode = self.members[parent].node;
        if let Some(i) = self.members[m].schema {
            return (pnode, i);
        }
        self.members[m].index = self.members[parent].child_initialized_count;
        let i = self.add_member_schema(parent);
        let mut disc = NO_DISCRIMINANT;
        if self.members[m].is_in_union {
            disc = self.members[parent].union_discriminant_count as u16;
            self.members[parent].union_discriminant_count += 1;
        }
        let (name, code_order, doc, start, end) = {
            let mi = &self.members[m];
            (mi.name.clone(), mi.code_order, mi.doc_comment.clone(), mi.start_byte, mi.end_byte)
        };
        {
            let f = &mut self.struct_of(pnode).fields[i];
            f.discriminant_value = disc;
            f.name = name;
            f.code_order = code_order as u16;
        }
        {
            let info = &mut self.info_mut(pnode).members.as_mut().unwrap()[i];
            info.doc_comment = doc;
            info.start_byte = start;
            info.end_byte = end;
        }
        self.members[m].schema = Some(i);
        (pnode, i)
    }

    fn finish_group(&mut self, m: usize) {
        let node_ref = self.members[m].node;
        if let Some(u) = self.members[m].union_scope {
            self.layout.add_discriminant(u);
            let count = self.members[m].union_discriminant_count as u16;
            let off = self.layout.discriminant_offset(u).expect("discriminant");
            let s = self.struct_of(node_ref);
            s.discriminant_count = count;
            s.discriminant_offset = off;
        }
        if let Some(parent) = self.members[m].parent {
            let pnode = self.members[parent].node;
            let parent_id = self.node_ref(pnode).id;
            let index = {
                // Make sure the field exists (assigns the index).
                self.get_schema(m);
                self.members[m].index
            };
            let group_id = generate_group_id(parent_id, index as u16);
            {
                let n = self.node_mut(node_ref);
                n.id = group_id;
                n.scope_id = parent_id;
            }
            let (pn, fi) = self.get_schema(m);
            self.struct_of(pn).fields[fi].kind = FieldKind::Group { type_id: group_id };
            let doc = self.members[m].doc_comment.clone();
            let info = self.info_mut(node_ref);
            info.id = group_id;
            if doc.is_some() {
                info.doc_comment = doc;
            }
        }
    }

    // ---- traversal of declarations -------------------------------------------------------

    fn traverse_union(
        &mut self,
        c: &mut Compiler,
        ts: &Translator,
        decl: &Declaration,
        members: &[Rc<Declaration>],
        parent: usize,
        union_id: usize,
        code_order: &mut u32,
    ) {
        if members.len() < 2 {
            c.add_error(ts.module, decl.span, "Union must have at least two members.");
        }
        for member in members {
            let mut ordinal: Option<u64> = None;
            let mut mi: Option<usize> = None;
            match &member.body {
                DeclBody::Field { .. } => {
                    self.members[parent].child_count += 1;
                    let g = self.layout.new_group(union_id);
                    let m = self.new_member(parent, *code_order, member, MKind::Field, g, NodeRef::Top, true);
                    *code_order += 1;
                    self.all_members.push(m);
                    ordinal = member.ordinal();
                    mi = Some(m);
                }
                DeclBody::Union => {
                    if member.name.value.is_empty() {
                        c.add_error(ts.module, member.span, "Unions cannot contain unnamed unions.");
                    } else {
                        self.members[parent].child_count += 1;
                        let g = self.layout.new_group(union_id);
                        let inner_union = self.layout.new_union(g);
                        let pnode = self.members[parent].node;
                        let gn = self.new_group_node(pnode, member);
                        let m = self.new_member(parent, *code_order, member, MKind::Union, g, gn, true);
                        *code_order += 1;
                        self.all_members.push(m);
                        self.members[m].union_scope = Some(inner_union);
                        let mut sub = 0;
                        self.traverse_union(c, ts, member, &member.nested, m, inner_union, &mut sub);
                        if let DeclId::Ordinal(o) = &member.id {
                            ordinal = Some(o.value);
                        }
                        mi = Some(m);
                    }
                }
                DeclBody::Group => {
                    self.members[parent].child_count += 1;
                    let g = self.layout.new_group(union_id);
                    let pnode = self.members[parent].node;
                    let gn = self.new_group_node(pnode, member);
                    let m = self.new_member(parent, *code_order, member, MKind::Group, g, gn, true);
                    *code_order += 1;
                    self.all_members.push(m);
                    self.traverse_group(c, ts, &member.nested, m, g);
                    mi = Some(m);
                }
                _ => {}
            }
            if let (Some(o), Some(m)) = (ordinal, mi) {
                self.members_by_ordinal.push((o, m));
            }
        }
    }

    fn traverse_group(
        &mut self,
        c: &mut Compiler,
        ts: &Translator,
        members: &[Rc<Declaration>],
        parent: usize,
        scope: Scope,
    ) {
        if members.is_empty() {
            let (s, e) = (self.members[parent].start_byte, self.members[parent].end_byte);
            c.add_error(ts.module, Span::new(s, e), "Group must have at least one member.");
        }
        self.traverse_top_or_group(c, ts, members, parent, scope);
    }

    fn traverse_top_or_group(
        &mut self,
        c: &mut Compiler,
        ts: &Translator,
        members: &[Rc<Declaration>],
        parent: usize,
        scope: Scope,
    ) {
        let mut code_order = 0u32;
        for member in members {
            let mut ordinal: Option<u64> = None;
            let mut mi: Option<usize> = None;
            match &member.body {
                DeclBody::Field { .. } => {
                    self.members[parent].child_count += 1;
                    let m = self.new_member(parent, code_order, member, MKind::Field, scope, NodeRef::Top, false);
                    code_order += 1;
                    self.all_members.push(m);
                    ordinal = member.ordinal();
                    mi = Some(m);
                }
                DeclBody::Union => {
                    let union_id = self.layout.new_union(scope);
                    let mut independent = 0u32;
                    let target;
                    let use_parent_order;
                    if member.name.value.is_empty() {
                        target = parent;
                        use_parent_order = true;
                    } else {
                        self.members[parent].child_count += 1;
                        let pnode = self.members[parent].node;
                        let gn = self.new_group_node(pnode, member);
                        let m = self.new_member(parent, code_order, member, MKind::Union, scope, gn, false);
                        code_order += 1;
                        self.all_members.push(m);
                        target = m;
                        use_parent_order = false;
                    }
                    self.members[target].union_scope = Some(union_id);
                    if use_parent_order {
                        self.traverse_union(c, ts, member, &member.nested, target, union_id, &mut code_order);
                    } else {
                        self.traverse_union(c, ts, member, &member.nested, target, union_id, &mut independent);
                    }
                    if let DeclId::Ordinal(o) = &member.id {
                        ordinal = Some(o.value);
                    }
                    mi = Some(target);
                }
                DeclBody::Group => {
                    self.members[parent].child_count += 1;
                    let pnode = self.members[parent].node;
                    let gn = self.new_group_node(pnode, member);
                    let m = self.new_member(parent, code_order, member, MKind::Group, scope, gn, false);
                    code_order += 1;
                    self.all_members.push(m);
                    self.traverse_group(c, ts, &member.nested, m, scope);
                }
                _ => {}
            }
            if let (Some(o), Some(m)) = (ordinal, mi) {
                self.members_by_ordinal.push((o, m));
            }
        }
    }

    fn translate_internal(&mut self, c: &mut Compiler, ts: &mut Translator, root: usize) {
        if !matches!(self.top.kind, NodeKind::Struct(_)) {
            self.top.kind = NodeKind::Struct(StructNode::default());
        }
        // stable sort by ordinal (multimap semantics)
        let mut order = std::mem::take(&mut self.members_by_ordinal);
        order.sort_by_key(|e| e.0);
        let mut dup = DupOrdinal::new();
        for (ordinal, m) in order.iter().copied() {
            if let Some(o) = self.members[m].ordinal.clone() {
                dup.check(c, ts.module, &o);
            }
            let (pn, fi) = self.get_schema(m);
            self.struct_of(pn).fields[fi].ordinal = Ordinal::Explicit(ordinal as u16);

            match self.members[m].kind {
                MKind::Field => {
                    let ty_expr = self.members[m].field_type.clone().expect("field type");
                    let implicit = ImplicitParams { scope_id: self.implicit.scope_id, params: self.implicit.params.clone() };
                    let ty = compile_type(c, ts, &ty_expr, &implicit);
                    let (ty, ok) = match ty {
                        Some(t) => (t, true),
                        None => (Type::Void, false),
                    };
                    let mut had_explicit = false;
                    let default_value;
                    if ok {
                        if let Some(dv) = self.members[m].field_default.clone() {
                            if self.members[m].is_param
                                && matches!(&dv.kind, ExprKind::RelativeName(n) if n == "null")
                            {
                                match ty {
                                    Type::Text | Type::Data | Type::List(_) | Type::Struct { .. } | Type::Interface { .. } | Type::AnyPointer(_) => {}
                                    _ => c.add_error(ts.module, dv.span, "Only pointer parameters can declare their default as 'null'."),
                                }
                                default_value = default_default_value(&ty);
                            } else {
                                default_value = compile_bootstrap_value(c, ts, &dv, &ty);
                            }
                            had_explicit = true;
                        } else {
                            default_value = default_default_value(&ty);
                        }
                    } else {
                        default_value = default_default_value(&ty);
                    }

                    // -1 void, -2 pointer
                    let lg: i32 = match &ty {
                        Type::Void => -1,
                        Type::Bool => 0,
                        Type::Int8 | Type::UInt8 => 3,
                        Type::Int16 | Type::UInt16 | Type::Enum { .. } => 4,
                        Type::Int32 | Type::UInt32 | Type::Float32 => 5,
                        Type::Int64 | Type::UInt64 | Type::Float64 => 6,
                        _ => -2,
                    };
                    let scope = self.members[m].field_scope;
                    let offset = if lg == -2 {
                        self.layout.add_pointer(scope)
                    } else if lg == -1 {
                        self.layout.add_void(scope);
                        0
                    } else {
                        self.layout.add_data(scope, lg as u32)
                    };
                    self.struct_of(pn).fields[fi].kind = FieldKind::Slot {
                        offset,
                        ty,
                        default_value,
                        had_explicit_default: had_explicit,
                    };
                }
                MKind::Union => {
                    let u = self.members[m].union_scope.unwrap();
                    if !self.layout.add_discriminant(u) {
                        if let Some(o) = &self.members[m].ordinal {
                            c.add_error(ts.module, o.span, "Union ordinal, if specified, must be greater than no more than one of its member ordinals (i.e. there can only be one field retroactively unionized).");
                        }
                    }
                }
                MKind::Group => panic!("Groups don't have ordinals."),
            }
        }

        self.finish_group(root);
        let all = self.all_members.clone();
        for m in all {
            let target;
            if self.members[m].is_param {
                target = Target::Param;
            } else {
                match self.members[m].kind {
                    MKind::Field => target = Target::Field,
                    MKind::Union => {
                        self.finish_group(m);
                        target = Target::Union;
                    }
                    MKind::Group => {
                        self.finish_group(m);
                        target = Target::Group;
                    }
                }
            }
            let anns = self.members[m].annotations.clone();
            let compiled = compile_annotation_applications(c, ts, &anns, target);
            let (pn, fi) = self.get_schema(m);
            self.struct_of(pn).fields[fi].annotations = compiled;
        }

        let (dw, pc) = (self.layout.data_word_count as u16, self.layout.pointer_count as u16);
        {
            let s = self.struct_of(NodeRef::Top);
            s.data_word_count = dw;
            s.pointer_count = pc;
        }
        for g in &mut self.groups {
            if let NodeKind::Struct(s) = &mut g.node.kind {
                s.data_word_count = dw;
                s.pointer_count = pc;
            }
        }
        if let Some(e) = self.layout.error.take() {
            c.add_error(ts.module, Span::new(ts.wip.start_byte, ts.wip.end_byte), e);
        }
    }
}

fn root_member(node: NodeRef) -> MemberInfo {
    MemberInfo {
        parent: None,
        code_order: 0,
        index: 0,
        child_count: 0,
        child_initialized_count: 0,
        union_discriminant_count: 0,
        is_in_union: false,
        name: String::new(),
        ordinal: None,
        kind: MKind::Group,
        is_param: false,
        field_type: None,
        field_default: None,
        annotations: vec![],
        start_byte: 0,
        end_byte: 0,
        doc_comment: None,
        schema: None,
        node,
        field_scope: Scope::Top,
        union_scope: None,
    }
}

fn compile_struct(c: &mut Compiler, ts: &mut Translator, members: &[Rc<Declaration>]) {
    let mut st = StructTranslator {
        layout: Layout::new(),
        members: vec![root_member(NodeRef::Top)],
        members_by_ordinal: vec![],
        all_members: vec![],
        top: ts.wip.clone(),
        top_info: ts.source_info.clone(),
        groups: vec![],
        implicit: ImplicitParams::none(),
    };
    st.top.kind = NodeKind::Struct(StructNode::default());
    st.traverse_top_or_group(c, ts, members, 0, Scope::Top);
    st.translate_internal(c, ts, 0);
    ts.wip = st.top;
    ts.source_info = st.top_info;
    ts.groups = st.groups;
}

/// Translate a method parameter/result list into a struct node.
fn translate_params(
    c: &mut Compiler,
    ts: &mut Translator,
    params: &[Param],
    top: Node,
    top_info: SourceInfo,
    implicit: ImplicitParams,
) -> (Node, SourceInfo) {
    let mut st = StructTranslator {
        layout: Layout::new(),
        members: vec![root_member(NodeRef::Top)],
        members_by_ordinal: vec![],
        all_members: vec![],
        top,
        top_info,
        groups: vec![],
        implicit,
    };
    st.top.kind = NodeKind::Struct(StructNode::default());
    for (i, p) in params.iter().enumerate() {
        st.members[0].child_count += 1;
        st.members.push(MemberInfo {
            parent: Some(0),
            code_order: i as u32,
            index: 0,
            child_count: 0,
            child_initialized_count: 0,
            union_discriminant_count: 0,
            is_in_union: false,
            name: p.name.value.clone(),
            ordinal: None,
            kind: MKind::Field,
            is_param: true,
            field_type: Some(p.ty.clone()),
            field_default: p.default_value.clone(),
            annotations: p.annotations.clone(),
            start_byte: p.span.start,
            end_byte: p.span.end,
            doc_comment: None,
            schema: None,
            node: NodeRef::Top,
            field_scope: Scope::Top,
            union_scope: None,
        });
        let m = st.members.len() - 1;
        st.all_members.push(m);
        st.members_by_ordinal.push((i as u64, m));
    }
    st.translate_internal(c, ts, 0);
    (st.top, st.top_info)
}

// ---------------------------------------------------------------------------------------------
// interfaces

fn compile_interface(
    c: &mut Compiler,
    ts: &mut Translator,
    superclasses: &[Expression],
    members: &[Rc<Declaration>],
) {
    let mut iface = InterfaceNode::default();
    for sc in superclasses {
        let mut s = Superclass { id: 0, brand: Brand::default() };
        if let Some(decl) = compile_decl_expr(c, ts, sc, &ImplicitParams::none()) {
            match decl.kind() {
                Some(DeclKind::Interface) => {
                    let (id, brand) = decl.id_and_brand(c);
                    s.id = id;
                    s.brand = brand;
                }
                Some(_) => decl.add_error(c, ts.module, format!("'{}' is not an interface.", decl.to_string())),
                None => decl.add_error(
                    c,
                    ts.module,
                    format!(
                        "'{}' is an unbound generic parameter. Currently we don't support extending these.",
                        decl.to_string()
                    ),
                ),
            }
        }
        iface.superclasses.push(s);
    }

    let mut methods: Vec<(u64, u32, Rc<Declaration>)> = vec![];
    let mut code_order = 0u32;
    for m in members {
        if matches!(m.body, DeclBody::Method { .. }) {
            methods.push((m.ordinal().unwrap_or(0), code_order, m.clone()));
            code_order += 1;
        }
    }
    methods.sort_by_key(|e| e.0);
    let mut infos = vec![];
    let mut dup = DupOrdinal::new();
    for (_, code_order, md) in &methods {
        let DeclBody::Method { params, results } = &md.body else { unreachable!() };
        let ord_decl = match &md.id {
            DeclId::Ordinal(o) => o.clone(),
            _ => Located::new(0, Span::default()),
        };
        dup.check(c, ts.module, &ord_decl);
        let ordinal = ord_decl.value as u16;
        infos.push(MemberSourceInfo { doc_comment: md.doc_comment.clone(), start_byte: md.span.start, end_byte: md.span.end });

        let implicit_names: Vec<String> = md.parameters.iter().map(|p| p.name.clone()).collect();
        if matches!(params.kind, ParamListKind::Stream) {
            c.add_error(ts.module, params.span, "'stream' can only appear after '->', not before.");
        }
        let (param_id, param_brand) =
            compile_param_list(c, ts, &md.name.value, ordinal, false, Some(params), &implicit_names);
        let (result_id, result_brand) =
            compile_param_list(c, ts, &md.name.value, ordinal, true, results.as_ref(), &implicit_names);
        let annotations = compile_annotation_applications(c, ts, &md.annotations, Target::Method);
        iface.methods.push(Method {
            name: md.name.value.clone(),
            code_order: *code_order as u16,
            implicit_parameters: implicit_names,
            param_struct_type: param_id,
            param_brand,
            result_struct_type: result_id,
            result_brand,
            annotations,
        });
    }
    ts.wip.kind = NodeKind::Interface(iface);
    ts.source_info.members = Some(infos);
}

pub const STREAM_RESULT_ID: u64 = 0x995f9a3377c0b16e;

fn compile_param_list(
    c: &mut Compiler,
    ts: &mut Translator,
    method_name: &str,
    ordinal: u16,
    is_results: bool,
    param_list: Option<&ParamList>,
    implicit: &[String],
) -> (Id, Brand) {
    let kind = param_list.map(|p| &p.kind);
    match kind {
        None => compile_named_param_list(c, ts, method_name, ordinal, is_results, &[], Span::default(), implicit),
        Some(ParamListKind::Named(ps)) => {
            compile_named_param_list(c, ts, method_name, ordinal, is_results, ps, param_list.unwrap().span, implicit)
        }
        Some(ParamListKind::Type(t)) => {
            let ip = ImplicitParams { scope_id: 0, params: implicit.to_vec() };
            if let Some(target) = compile_decl_expr(c, ts, t, &ip) {
                match target.kind() {
                    Some(DeclKind::Struct) => return target.id_and_brand(c),
                    Some(_) => c.add_error(
                        ts.module,
                        t.span,
                        format!("'{}' is not a struct type.", expression_string(t)),
                    ),
                    None => {
                        target.add_error(c, ts.module, "Cannot use generic parameter as whole input or output of a method. Instead, use a parameter/result list containing a field with this type.");
                        return (0, Brand::default());
                    }
                }
            }
            (0, Brand::default())
        }
        Some(ParamListKind::Stream) => {
            let span = param_list.unwrap().span;
            match c.resolve_import(ts.node_idx, "/capnp/stream.capnp") {
                Some(stream) => {
                    if c.resolve_member(stream.resolver, "StreamResult").is_none() {
                        c.add_error(ts.module, span, "The version of '/capnp/stream.capnp' found in your import path does not appear to be the official one; it is missing the declaration of StreamResult.");
                    }
                }
                None => c.add_error(ts.module, span, "A method declaration uses streaming, but '/capnp/stream.capnp' is not found in the import path. This is a standard file that should always be installed with the Cap'n Proto compiler."),
            }
            (STREAM_RESULT_ID, Brand::default())
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn compile_named_param_list(
    c: &mut Compiler,
    ts: &mut Translator,
    method_name: &str,
    ordinal: u16,
    is_results: bool,
    params: &[Param],
    span: Span,
    implicit: &[String],
) -> (Id, Brand) {
    let type_name = format!("{}{}", method_name, if is_results { "$Results" } else { "$Params" });
    let id = generate_method_params_id(ts.wip.id, ordinal, is_results);
    let display = format!("{}.{}", ts.wip.display_name, type_name);
    let node = Node {
        id,
        display_name: display.clone(),
        display_name_prefix_length: (display.len() - type_name.len()) as u32,
        scope_id: 0,
        nested_nodes: None,
        annotations: vec![],
        parameters: vec![],
        is_generic: ts.wip.is_generic || !implicit.is_empty(),
        start_byte: span.start,
        end_byte: span.end,
        kind: NodeKind::Struct(StructNode::default()),
    };
    let info = SourceInfo { id, start_byte: span.start, end_byte: span.end, ..Default::default() };
    let (mut node, info) = translate_params(
        c,
        ts,
        params,
        node,
        info,
        ImplicitParams { scope_id: id, params: implicit.to_vec() },
    );
    let brand = ts.local_brand.push(id, implicit.len());
    if !implicit.is_empty() {
        let decls: Vec<BrandedDecl> = (0..implicit.len()).map(BrandedDecl::implicit_method_param).collect();
        node.parameters = implicit.to_vec();
        // Mirrors the reference implementation, which discards the resulting scope.
        let _ = brand.set_params(c, decls, DeclKind::Struct, &Expression { kind: ExprKind::Unknown, span: Span::default() });
    }
    let b = brand.compile(c);
    ts.param_structs.push(AuxNode { node, source_info: info });
    (id, b)
}

// ---------------------------------------------------------------------------------------------
// finishing

pub fn finish_translator(c: &mut Compiler, idx: NodeIdx, mut ts: Translator) -> (Translator, NodeSetRc) {
    let _ = idx;
    let mut results: Vec<Value> = vec![];
    let mut i = 0;
    while i < ts.unfinished.len() {
        let (src, ty, default) = {
            let u = &ts.unfinished[i];
            (u.source.clone(), u.ty.clone(), u.default.clone())
        };
        let v = crate::value::compile_value(c, &mut ts, &src, &ty, false).unwrap_or(default);
        results.push(v);
        i += 1;
    }
    patch_node(&mut ts.wip, &results);
    for g in &mut ts.groups {
        patch_node(&mut g.node, &results);
    }
    for p in &mut ts.param_structs {
        patch_node(&mut p.node, &results);
    }
    let set = ts.bootstrap_set();
    (ts, set)
}

fn patch_value(v: &mut Value, results: &[Value]) {
    if let Value::Pending(k) = v {
        *v = results[*k].clone();
    }
}

fn patch_annotations(anns: &mut [Annotation], results: &[Value]) {
    for a in anns {
        patch_value(&mut a.value, results);
    }
}

fn patch_node(n: &mut Node, results: &[Value]) {
    patch_annotations(&mut n.annotations, results);
    match &mut n.kind {
        NodeKind::Const { value, .. } => patch_value(value, results),
        NodeKind::Struct(s) => {
            for f in &mut s.fields {
                patch_annotations(&mut f.annotations, results);
                if let FieldKind::Slot { default_value, .. } = &mut f.kind {
                    patch_value(default_value, results);
                }
            }
        }
        NodeKind::Enum(es) => {
            for e in es {
                patch_annotations(&mut e.annotations, results);
            }
        }
        NodeKind::Interface(i) => {
            for m in &mut i.methods {
                patch_annotations(&mut m.annotations, results);
            }
        }
        _ => {}
    }
}

#[allow(dead_code)]
pub fn compile_type_for_value() {}
