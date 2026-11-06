//! The compiler core (counterpart of `compiler.c++`): the declaration tree, lazy node states,
//! name resolution, `using` aliases, dependency traversal and the final node list.

use crate::ast::*;
use crate::error::{Diagnostic, Error, Span};
use crate::generics::BrandScope;
use crate::ids::generate_child_id;
use crate::schema::{self, Brand, Id, Node, SourceInfo};
use crate::source::{Dir, SourceLoader};
use crate::translator::Translator;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

// Eagerness flags for traversal.
pub const NODE: u32 = 1;
pub const PARENTS: u32 = 2;
pub const CHILDREN: u32 = 4;
pub const DEPENDENCIES: u32 = 1 << 15;
pub const DEPENDENCY_PARENTS: u32 = PARENTS * DEPENDENCIES;
pub const DEFAULT_EAGERNESS: u32 = NODE | CHILDREN | DEPENDENCIES | DEPENDENCY_PARENTS;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DeclKind {
    File,
    Using,
    Const,
    Enum,
    Enumerant,
    Struct,
    Field,
    Union,
    Group,
    Interface,
    Method,
    Annotation,
    BuiltinVoid,
    BuiltinBool,
    BuiltinInt8,
    BuiltinInt16,
    BuiltinInt32,
    BuiltinInt64,
    BuiltinUInt8,
    BuiltinUInt16,
    BuiltinUInt32,
    BuiltinUInt64,
    BuiltinFloat32,
    BuiltinFloat64,
    BuiltinText,
    BuiltinData,
    BuiltinList,
    BuiltinObject,
    BuiltinAnyPointer,
    BuiltinAnyStruct,
    BuiltinAnyList,
    BuiltinCapability,
}

pub fn decl_kind(d: &Declaration) -> DeclKind {
    match &d.body {
        DeclBody::File => DeclKind::File,
        DeclBody::Using { .. } => DeclKind::Using,
        DeclBody::Const { .. } => DeclKind::Const,
        DeclBody::Enum => DeclKind::Enum,
        DeclBody::Enumerant => DeclKind::Enumerant,
        DeclBody::Struct => DeclKind::Struct,
        DeclBody::Field { .. } => DeclKind::Field,
        DeclBody::Union => DeclKind::Union,
        DeclBody::Group => DeclKind::Group,
        DeclBody::Interface { .. } => DeclKind::Interface,
        DeclBody::Method { .. } => DeclKind::Method,
        DeclBody::Annotation { .. } => DeclKind::Annotation,
    }
}

const BUILTINS: &[(&str, DeclKind, usize)] = &[
    ("Void", DeclKind::BuiltinVoid, 0),
    ("Bool", DeclKind::BuiltinBool, 0),
    ("Int8", DeclKind::BuiltinInt8, 0),
    ("Int16", DeclKind::BuiltinInt16, 0),
    ("Int32", DeclKind::BuiltinInt32, 0),
    ("Int64", DeclKind::BuiltinInt64, 0),
    ("UInt8", DeclKind::BuiltinUInt8, 0),
    ("UInt16", DeclKind::BuiltinUInt16, 0),
    ("UInt32", DeclKind::BuiltinUInt32, 0),
    ("UInt64", DeclKind::BuiltinUInt64, 0),
    ("Float32", DeclKind::BuiltinFloat32, 0),
    ("Float64", DeclKind::BuiltinFloat64, 0),
    ("Text", DeclKind::BuiltinText, 0),
    ("Data", DeclKind::BuiltinData, 0),
    ("List", DeclKind::BuiltinList, 1),
    ("Object", DeclKind::BuiltinObject, 0),
    ("AnyPointer", DeclKind::BuiltinAnyPointer, 0),
    ("AnyStruct", DeclKind::BuiltinAnyStruct, 0),
    ("AnyList", DeclKind::BuiltinAnyList, 0),
    ("Capability", DeclKind::BuiltinCapability, 0),
];

#[derive(Clone, Debug)]
pub struct ResolvedDecl {
    pub id: Id,
    pub generic_param_count: usize,
    pub scope_id: Id,
    pub kind: DeclKind,
    pub resolver: usize,
    pub brand: Option<Brand>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedParameter {
    pub id: Id,
    pub index: usize,
}

#[derive(Clone, Debug)]
pub enum ResolveResult {
    Decl(ResolvedDecl),
    Param(ResolvedParameter),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum State {
    Stub,
    Expanded,
    Bootstrap,
    Finished,
}

/// A translated declaration: node plus auxiliary nodes (groups / method parameter structs).
#[derive(Clone, Debug)]
pub struct NodeSetRc {
    pub node: Rc<Node>,
    pub aux: Vec<Rc<Node>>,
    pub source_info: Vec<SourceInfo>,
}

pub struct NodeData {
    pub module: usize,
    pub parent: Option<usize>,
    pub decl: Option<Rc<Declaration>>,
    pub id: Id,
    pub display_name: String,
    pub kind: DeclKind,
    pub generic_param_count: usize,
    pub is_builtin: bool,
    pub start_byte: u32,
    pub end_byte: u32,
    pub state: State,
    pub in_get_content: bool,
    pub nested: Vec<(String, usize)>,
    pub aliases: Vec<(String, usize)>,
    pub translator: Option<Translator>,
    pub bootstrap: Option<NodeSetRc>,
    pub final_set: Option<NodeSetRc>,
    /// The final node set has been loaded into the (simulated) final schema loader.
    pub loaded: bool,
}

struct AliasData {
    module: usize,
    parent: usize,
    target_name: Expression,
    target: Option<ResolveResult>,
    initialized: bool,
}

/// Simulates the insertion order of the official `SchemaLoader` (a hash map that iterates in
/// insertion order, where dependencies are inserted as placeholders while a node is validated).
#[derive(Default)]
pub struct LoaderSim {
    order: Vec<Id>,
    present: HashSet<Id>,
    loaded: HashMap<Id, Rc<Node>>,
}

impl LoaderSim {
    fn touch(&mut self, id: Id) {
        if self.present.insert(id) {
            self.order.push(id);
        }
    }

    fn type_ids(ty: &schema::Type, out: &mut Vec<Id>) {
        use schema::Type::*;
        match ty {
            List(e) => Self::type_ids(e, out),
            Enum { type_id, brand } | Struct { type_id, brand } | Interface { type_id, brand } => {
                out.push(*type_id);
                Self::brand_ids(brand, out);
            }
            _ => {}
        }
    }

    fn brand_ids(brand: &Brand, out: &mut Vec<Id>) {
        for s in &brand.scopes {
            if let schema::BrandScopeKind::Bind(b) = &s.kind {
                for x in b {
                    if let schema::Binding::Type(t) = x {
                        Self::type_ids(t, out);
                    }
                }
            }
        }
    }

    /// Ids touched while validating `node`, in order.
    fn validation_ids(node: &Node) -> Vec<Id> {
        let mut out = vec![];
        match &node.kind {
            schema::NodeKind::Struct(s) => {
                for f in &s.fields {
                    match &f.kind {
                        schema::FieldKind::Slot { ty, .. } => Self::type_ids(ty, &mut out),
                        schema::FieldKind::Group { type_id } => out.push(*type_id),
                    }
                }
                if s.is_group {
                    out.push(node.scope_id);
                }
            }
            schema::NodeKind::Interface(i) => {
                for s in &i.superclasses {
                    out.push(s.id);
                    Self::brand_ids(&s.brand, &mut out);
                }
                for m in &i.methods {
                    out.push(m.param_struct_type);
                    Self::brand_ids(&m.param_brand, &mut out);
                    out.push(m.result_struct_type);
                    Self::brand_ids(&m.result_brand, &mut out);
                }
            }
            schema::NodeKind::Const { ty, .. } | schema::NodeKind::Annotation { ty, .. } => {
                Self::type_ids(ty, &mut out)
            }
            _ => {}
        }
        out
    }

    pub fn load_once(&mut self, node: &Rc<Node>) {
        if self.loaded.contains_key(&node.id) {
            return;
        }
        for id in Self::validation_ids(node) {
            self.touch(id);
        }
        self.touch(node.id);
        self.loaded.insert(node.id, node.clone());
    }

    /// All really loaded nodes, in insertion order.
    pub fn all_loaded(&self) -> Vec<Rc<Node>> {
        self.order
            .iter()
            .filter_map(|id| self.loaded.get(id).cloned())
            .collect()
    }
}

pub struct RequestedFile {
    pub id: Id,
    pub filename: String,
    pub imports: Vec<(Id, String)>,
    pub module: usize,
}

pub struct Compiler {
    pub loader: SourceLoader,
    pub nodes: Vec<NodeData>,
    nodes_by_id: HashMap<Id, usize>,
    aliases: Vec<AliasData>,
    builtins: HashMap<String, usize>,
    builtins_by_kind: HashMap<DeclKind, usize>,
    module_roots: HashMap<usize, usize>,
    next_bogus_id: u64,
    pub diagnostics: Vec<Diagnostic>,
    pub bootstrap_by_id: HashMap<Id, Rc<Node>>,
    pub final_by_id: HashMap<Id, Rc<Node>>,
    pub sim: LoaderSim,
    pub source_info_inserted: Vec<SourceInfo>,
    source_info_ids: HashSet<Id>,
    pub requested: Vec<RequestedFile>,
}

pub type NodeIdx = usize;

impl Compiler {
    pub fn new(loader: SourceLoader) -> Self {
        let mut c = Compiler {
            loader,
            nodes: vec![],
            nodes_by_id: HashMap::new(),
            aliases: vec![],
            builtins: HashMap::new(),
            builtins_by_kind: HashMap::new(),
            module_roots: HashMap::new(),
            next_bogus_id: 1000,
            diagnostics: vec![],
            bootstrap_by_id: HashMap::new(),
            final_by_id: HashMap::new(),
            sim: LoaderSim::default(),
            source_info_inserted: vec![],
            source_info_ids: HashSet::new(),
            requested: vec![],
        };
        for (i, (name, kind, gpc)) in BUILTINS.iter().enumerate() {
            let idx = c.nodes.len();
            c.nodes.push(NodeData {
                module: usize::MAX,
                parent: None,
                decl: None,
                id: 1014 + i as u64,
                display_name: name.to_string(),
                kind: *kind,
                generic_param_count: *gpc,
                is_builtin: true,
                start_byte: 0,
                end_byte: 0,
                state: State::Stub,
                in_get_content: false,
                nested: vec![],
                aliases: vec![],
                translator: None,
                bootstrap: None,
                final_set: None,
                loaded: false,
            });
            c.builtins.insert(name.to_string(), idx);
            c.builtins_by_kind.insert(*kind, idx);
        }
        c
    }

    // ------------------------------------------------------------------ errors

    pub fn add_error(&mut self, module: usize, span: Span, message: impl Into<String>) {
        let file = self
            .loader
            .modules
            .get(module)
            .map(|m| m.display_name.clone())
            .unwrap_or_default();
        self.diagnostics.push(Diagnostic {
            file,
            span,
            message: message.into(),
        });
    }

    pub fn had_errors(&self) -> bool {
        !self.diagnostics.is_empty()
    }

    pub fn report_resolution(&mut self, module: usize, start: u32, end: u32, type_id: Id) {
        if let Some(m) = self.loader.modules.get_mut(module) {
            m.resolutions.push(crate::source::Resolution {
                start_byte: start,
                end_byte: end,
                type_id,
            });
        }
    }

    // ------------------------------------------------------------------ modules and nodes

    /// Make sure `module` has a root node and return it.
    pub fn add_module(&mut self, module: usize) -> NodeIdx {
        if let Some(&r) = self.module_roots.get(&module) {
            return r;
        }
        let decl = self.loader.modules[module].root.clone();
        let display = self.loader.modules[module].display_name.clone();
        let id = decl.uid().unwrap_or(0);
        let idx = self.nodes.len();
        self.nodes.push(NodeData {
            module,
            parent: None,
            kind: DeclKind::File,
            generic_param_count: decl.parameters.len(),
            is_builtin: false,
            start_byte: decl.span.start,
            end_byte: decl.span.end,
            state: State::Stub,
            in_get_content: false,
            nested: vec![],
            aliases: vec![],
            translator: None,
            bootstrap: None,
            final_set: None,
            loaded: false,
            id,
            display_name: display,
            decl: Some(decl),
        });
        self.module_roots.insert(module, idx);
        let final_id = self.add_node(id, idx);
        self.nodes[idx].id = final_id;
        idx
    }

    fn new_child(&mut self, parent: NodeIdx, decl: Rc<Declaration>) -> NodeIdx {
        let module = self.nodes[parent].module;
        let pid = self.nodes[parent].id;
        let id = decl
            .uid()
            .unwrap_or_else(|| generate_child_id(pid, &decl.name.value));
        let sep = if self.nodes[parent].parent.is_none() {
            ':'
        } else {
            '.'
        };
        let display = format!(
            "{}{}{}",
            self.nodes[parent].display_name, sep, decl.name.value
        );
        let span = if !decl.name.value.is_empty() {
            decl.name.span
        } else {
            decl.span
        };
        let idx = self.nodes.len();
        self.nodes.push(NodeData {
            module,
            parent: Some(parent),
            kind: decl_kind(&decl),
            generic_param_count: decl.parameters.len(),
            is_builtin: false,
            start_byte: span.start,
            end_byte: span.end,
            state: State::Stub,
            in_get_content: false,
            nested: vec![],
            aliases: vec![],
            translator: None,
            bootstrap: None,
            final_set: None,
            loaded: false,
            id,
            display_name: display,
            decl: Some(decl),
        });
        let final_id = self.add_node(id, idx);
        self.nodes[idx].id = final_id;
        idx
    }

    fn add_node(&mut self, desired: Id, idx: NodeIdx) -> Id {
        let mut desired = desired;
        loop {
            if let std::collections::hash_map::Entry::Vacant(e) = self.nodes_by_id.entry(desired) {
                e.insert(idx);
                return desired;
            }
            if desired & (1u64 << 63) != 0 {
                let other = self.nodes_by_id[&desired];
                let (m, s, e) = (
                    self.nodes[idx].module,
                    self.nodes[idx].start_byte,
                    self.nodes[idx].end_byte,
                );
                self.add_error(m, Span::new(s, e), format!("Duplicate ID @0x{desired:x}."));
                let (m2, s2, e2) = (
                    self.nodes[other].module,
                    self.nodes[other].start_byte,
                    self.nodes[other].end_byte,
                );
                self.add_error(
                    m2,
                    Span::new(s2, e2),
                    format!("ID @0x{desired:x} originally used here."),
                );
            }
            desired = self.next_bogus_id;
            self.next_bogus_id += 1;
        }
    }

    pub fn find_node(&self, id: Id) -> Option<NodeIdx> {
        self.nodes_by_id.get(&id).copied()
    }

    pub fn decl(&self, idx: NodeIdx) -> Rc<Declaration> {
        self.nodes[idx]
            .decl
            .clone()
            .expect("builtin has no declaration")
    }

    pub fn node_error(&mut self, idx: NodeIdx, msg: impl Into<String>) {
        let (m, s, e) = (
            self.nodes[idx].module,
            self.nodes[idx].start_byte,
            self.nodes[idx].end_byte,
        );
        self.add_error(m, Span::new(s, e), msg);
    }

    /// Advance the node to at least `min`. Returns false when a recursive dependency was detected.
    pub fn ensure_state(&mut self, idx: NodeIdx, min: State) -> bool {
        assert!(
            !self.nodes[idx].is_builtin,
            "illegal method call for built-in declaration"
        );
        if self.nodes[idx].state >= min {
            return true;
        }
        if self.nodes[idx].in_get_content {
            self.node_error(idx, "Declaration recursively depends on itself.");
            return false;
        }
        self.nodes[idx].in_get_content = true;

        if self.nodes[idx].state == State::Stub && min > State::Stub {
            self.expand(idx);
        }
        if self.nodes[idx].state == State::Expanded && min > State::Expanded {
            let t = crate::translator::translate_node(self, idx);
            self.register_bootstrap(idx, &t);
            self.nodes[idx].translator = Some(t);
            self.nodes[idx].state = State::Bootstrap;
        }
        if self.nodes[idx].state == State::Bootstrap && min > State::Bootstrap {
            let t = self.nodes[idx].translator.take().expect("translator");
            let (t, set) = crate::translator::finish_translator(self, idx, t);
            self.register_final(idx, &set);
            self.nodes[idx].translator = Some(t);
            self.nodes[idx].final_set = Some(set);
            self.nodes[idx].state = State::Finished;
        }
        self.nodes[idx].in_get_content = false;
        true
    }

    fn register_bootstrap(&mut self, idx: NodeIdx, t: &Translator) {
        let set = t.bootstrap_set();
        self.bootstrap_by_id.insert(set.node.id, set.node.clone());
        for a in &set.aux {
            self.bootstrap_by_id.insert(a.id, a.clone());
        }
        self.nodes[idx].bootstrap = Some(set);
    }

    fn register_final(&mut self, _idx: NodeIdx, set: &NodeSetRc) {
        self.final_by_id.insert(set.node.id, set.node.clone());
        for a in &set.aux {
            self.final_by_id.insert(a.id, a.clone());
        }
    }

    fn expand(&mut self, idx: NodeIdx) {
        let decl = self.decl(idx);
        for nested in &decl.nested {
            match &nested.body {
                DeclBody::File
                | DeclBody::Const { .. }
                | DeclBody::Annotation { .. }
                | DeclBody::Enum
                | DeclBody::Struct
                | DeclBody::Interface { .. } => {
                    let child = self.new_child(idx, nested.clone());
                    self.nodes[idx]
                        .nested
                        .push((nested.name.value.clone(), child));
                }
                DeclBody::Using { target } => {
                    let a = self.aliases.len();
                    self.aliases.push(AliasData {
                        module: self.nodes[idx].module,
                        parent: idx,
                        target_name: target.clone(),
                        target: None,
                        initialized: false,
                    });
                    self.nodes[idx].aliases.push((nested.name.value.clone(), a));
                }
                _ => {}
            }
        }
        self.nodes[idx].state = State::Expanded;
    }

    /// Bootstrap schema of a node by id (also finds group / parameter nodes of finished owners).
    pub fn resolve_bootstrap_schema(&mut self, id: Id) -> Option<Rc<Node>> {
        if let Some(idx) = self.find_node(id) {
            if let Some(f) = self.final_by_id.get(&id) {
                return Some(f.clone());
            }
            if !self.ensure_state(idx, State::Bootstrap) {
                return None;
            }
            return self.bootstrap_by_id.get(&id).cloned();
        }
        self.final_by_id
            .get(&id)
            .or_else(|| self.bootstrap_by_id.get(&id))
            .cloned()
    }

    pub fn resolve_final_schema(&mut self, id: Id) -> Option<Rc<Node>> {
        if let Some(idx) = self.find_node(id) {
            if !self.ensure_state(idx, State::Finished) {
                return None;
            }
            return self.final_by_id.get(&id).cloned();
        }
        self.final_by_id.get(&id).cloned()
    }

    // ------------------------------------------------------------------ resolver

    fn resolved(&self, idx: NodeIdx, scope_id: Id) -> ResolvedDecl {
        let n = &self.nodes[idx];
        ResolvedDecl {
            id: n.id,
            generic_param_count: n.generic_param_count,
            scope_id,
            kind: n.kind,
            resolver: idx,
            brand: None,
        }
    }

    pub fn resolve(&mut self, node: NodeIdx, name: &str) -> Option<ResolveResult> {
        if let Some(r) = self.resolve_member(node, name) {
            return Some(r);
        }
        if let Some(d) = &self.nodes[node].decl {
            for (i, p) in d.parameters.iter().enumerate() {
                if p.name == name {
                    return Some(ResolveResult::Param(ResolvedParameter {
                        id: self.nodes[node].id,
                        index: i,
                    }));
                }
            }
        }
        if let Some(p) = self.nodes[node].parent {
            self.resolve(p, name)
        } else if let Some(&b) = self.builtins.get(name) {
            Some(ResolveResult::Decl(self.resolved(b, 0)))
        } else {
            None
        }
    }

    pub fn resolve_member(&mut self, node: NodeIdx, name: &str) -> Option<ResolveResult> {
        if self.nodes[node].is_builtin {
            return None;
        }
        if !self.ensure_state(node, State::Expanded) {
            return None;
        }
        if let Some((_, c)) = self.nodes[node].nested.iter().find(|(n, _)| n == name) {
            let c = *c;
            let sid = self.nodes[node].id;
            return Some(ResolveResult::Decl(self.resolved(c, sid)));
        }
        if let Some((_, a)) = self.nodes[node].aliases.iter().find(|(n, _)| n == name) {
            let a = *a;
            return self.alias_compile(a);
        }
        None
    }

    pub fn resolve_builtin(&self, kind: DeclKind) -> ResolvedDecl {
        self.resolved(self.builtins_by_kind[&kind], 0)
    }

    pub fn resolve_id(&self, id: Id) -> ResolvedDecl {
        let n = self.find_node(id).expect("resolve_id: unknown id");
        let pid = self.nodes[n].parent.map(|p| self.nodes[p].id).unwrap_or(0);
        self.resolved(n, pid)
    }

    pub fn get_parent(&self, node: NodeIdx) -> Option<ResolvedDecl> {
        self.nodes[node].parent.map(|p| {
            let scope = self.nodes[p].parent.map(|g| self.nodes[g].id).unwrap_or(0);
            self.resolved(p, scope)
        })
    }

    pub fn get_top_scope(&self, node: NodeIdx) -> ResolvedDecl {
        let root = self.module_roots[&self.nodes[node].module];
        self.resolved(root, 0)
    }

    pub fn resolve_import(&mut self, node: NodeIdx, name: &str) -> Option<ResolvedDecl> {
        let module = self.nodes[node].module;
        let mut errs = vec![];
        let m = self.loader.import_relative(module, name, &mut errs);
        self.diagnostics.extend(errs);
        let m = m?;
        let root = self.add_module(m);
        Some(self.resolved(root, 0))
    }

    pub fn read_embed(&self, node: NodeIdx, name: &str) -> Option<Vec<u8>> {
        self.loader.embed_relative(self.nodes[node].module, name)
    }

    pub fn alias_compile(&mut self, a: usize) -> Option<ResolveResult> {
        if !self.aliases[a].initialized {
            self.aliases[a].initialized = true;
            let (module, parent, expr) = (
                self.aliases[a].module,
                self.aliases[a].parent,
                self.aliases[a].target_name.clone(),
            );
            let pid = self.nodes[parent].id;
            let pcount = self.nodes[parent].generic_param_count;
            let r = self.compile_decl(pid, pcount, parent, module, &expr);
            self.aliases[a].target = r;
        }
        self.aliases[a].target.clone()
    }

    /// `NodeTranslator::compileDecl`.
    pub fn compile_decl(
        &mut self,
        scope_id: Id,
        scope_param_count: usize,
        resolver: NodeIdx,
        module: usize,
        expr: &Expression,
    ) -> Option<ResolveResult> {
        let scope = BrandScope::new_root(self, module, scope_id, scope_param_count, resolver);
        let decl = scope.compile_decl_expression(
            self,
            expr,
            resolver,
            &crate::generics::ImplicitParams::none(),
        )?;
        Some(decl.as_resolve_result(self, scope.leaf_id()))
    }

    // ------------------------------------------------------------------ traversal

    pub fn eagerly_compile(&mut self, root: NodeIdx, eagerness: u32) {
        let mut seen: HashMap<NodeIdx, u32> = HashMap::new();
        let mut infos: Vec<SourceInfo> = vec![];
        self.traverse(root, eagerness, &mut seen, &mut infos);
        for si in infos {
            if self.source_info_ids.insert(si.id) {
                self.source_info_inserted.push(si);
            }
        }
    }

    fn load_final_schema(&mut self, idx: NodeIdx) {
        if self.nodes[idx].loaded {
            return;
        }
        if let Some(set) = self.nodes[idx].final_set.clone() {
            for a in &set.aux {
                self.sim.load_once(a);
            }
            self.sim.load_once(&set.node);
            self.nodes[idx].loaded = true;
        }
    }

    fn traverse(
        &mut self,
        idx: NodeIdx,
        eagerness: u32,
        seen: &mut HashMap<NodeIdx, u32>,
        si: &mut Vec<SourceInfo>,
    ) {
        let slot = seen.entry(idx).or_insert(0);
        if *slot & eagerness == eagerness {
            return;
        }
        *slot |= eagerness;

        if self.ensure_state(idx, State::Finished) {
            self.load_final_schema(idx);
            if let Some(set) = self.nodes[idx].final_set.clone() {
                if eagerness / DEPENDENCIES != 0 {
                    let new_e = (eagerness & !(DEPENDENCIES - 1)) | (eagerness / DEPENDENCIES);
                    self.traverse_node_dependencies(&set.node, new_e, seen, si);
                    for a in &set.aux {
                        self.traverse_node_dependencies(a, new_e, seen, si);
                    }
                }
                si.extend(set.source_info.iter().cloned());
            }
        }

        if eagerness & PARENTS != 0 {
            if let Some(p) = self.nodes[idx].parent {
                self.traverse(p, eagerness, seen, si);
            }
        }

        if eagerness & CHILDREN != 0 && self.ensure_state(idx, State::Expanded) {
            let children: Vec<usize> = self.nodes[idx].nested.iter().map(|(_, c)| *c).collect();
            for c in children {
                self.traverse(c, eagerness, seen, si);
            }
            let aliases: Vec<usize> = self.nodes[idx].aliases.iter().map(|(_, a)| *a).collect();
            for a in aliases {
                self.alias_compile(a);
            }
        }
    }

    fn traverse_node_dependencies(
        &mut self,
        node: &Node,
        e: u32,
        seen: &mut HashMap<NodeIdx, u32>,
        si: &mut Vec<SourceInfo>,
    ) {
        use schema::NodeKind::*;
        match &node.kind {
            Struct(s) => {
                for f in &s.fields {
                    if let schema::FieldKind::Slot { ty, .. } = &f.kind {
                        self.traverse_type(ty, e, seen, si);
                    }
                    self.traverse_annotations(&f.annotations, e, seen, si);
                }
            }
            Enum(en) => {
                for x in en {
                    self.traverse_annotations(&x.annotations, e, seen, si);
                }
            }
            Interface(i) => {
                for s in &i.superclasses {
                    if s.id != 0 {
                        self.traverse_dependency(s.id, e, seen, si, false);
                    }
                    self.traverse_brand(&s.brand, e, seen, si);
                }
                for m in &i.methods {
                    self.traverse_dependency(m.param_struct_type, e, seen, si, true);
                    self.traverse_brand(&m.param_brand, e, seen, si);
                    self.traverse_dependency(m.result_struct_type, e, seen, si, true);
                    self.traverse_brand(&m.result_brand, e, seen, si);
                    self.traverse_annotations(&m.annotations, e, seen, si);
                }
            }
            Const { ty, .. } => self.traverse_type(ty, e, seen, si),
            Annotation { ty, .. } => self.traverse_type(ty, e, seen, si),
            File => {}
        }
        self.traverse_annotations(&node.annotations, e, seen, si);
    }

    fn traverse_type(
        &mut self,
        ty: &schema::Type,
        e: u32,
        seen: &mut HashMap<NodeIdx, u32>,
        si: &mut Vec<SourceInfo>,
    ) {
        use schema::Type::*;
        match ty {
            Struct { type_id, brand } | Enum { type_id, brand } | Interface { type_id, brand } => {
                self.traverse_dependency(*type_id, e, seen, si, false);
                self.traverse_brand(brand, e, seen, si);
            }
            List(el) => self.traverse_type(el, e, seen, si),
            _ => {}
        }
    }

    fn traverse_brand(
        &mut self,
        brand: &Brand,
        e: u32,
        seen: &mut HashMap<NodeIdx, u32>,
        si: &mut Vec<SourceInfo>,
    ) {
        for s in &brand.scopes {
            if let schema::BrandScopeKind::Bind(bs) = &s.kind {
                for b in bs {
                    if let schema::Binding::Type(t) = b {
                        self.traverse_type(t, e, seen, si);
                    }
                }
            }
        }
    }

    fn traverse_annotations(
        &mut self,
        anns: &[schema::Annotation],
        e: u32,
        seen: &mut HashMap<NodeIdx, u32>,
        si: &mut Vec<SourceInfo>,
    ) {
        for a in anns {
            if let Some(n) = self.find_node(a.id) {
                self.traverse(n, e, seen, si);
            }
        }
    }

    fn traverse_dependency(
        &mut self,
        id: Id,
        e: u32,
        seen: &mut HashMap<NodeIdx, u32>,
        si: &mut Vec<SourceInfo>,
        ignore_if_not_found: bool,
    ) {
        if let Some(n) = self.find_node(id) {
            self.traverse(n, e, seen, si);
        } else if !ignore_if_not_found {
            // Dependency ID not present in compiler; nothing to traverse.
        }
    }

    // ------------------------------------------------------------------ imports table

    pub fn file_import_table(&mut self, module: usize) -> Vec<(Id, String)> {
        let mut names = BTreeSet::new();
        find_imports_decl(&self.loader.modules[module].root.clone(), &mut names);
        let mut out = vec![];
        for n in names {
            let mut errs = vec![];
            if let Some(m) = self.loader.import_relative(module, &n, &mut errs) {
                let root = self.add_module(m);
                out.push((self.nodes[root].id, n));
            }
        }
        out
    }
}

fn find_imports_expr(e: &Expression, out: &mut BTreeSet<String>) {
    match &e.kind {
        ExprKind::Import(n) => {
            out.insert(n.clone());
        }
        ExprKind::List(l) => l.iter().for_each(|x| find_imports_expr(x, out)),
        ExprKind::Tuple(t) => t.iter().for_each(|p| find_imports_expr(&p.value, out)),
        ExprKind::Application { function, params } => {
            find_imports_expr(function, out);
            params.iter().for_each(|p| find_imports_expr(&p.value, out));
        }
        ExprKind::Member { parent, .. } => find_imports_expr(parent, out),
        _ => {}
    }
}

fn find_imports_params(pl: &ParamList, out: &mut BTreeSet<String>) {
    match &pl.kind {
        ParamListKind::Named(ps) => {
            for p in ps {
                find_imports_expr(&p.ty, out);
                for a in &p.annotations {
                    find_imports_expr(&a.name, out);
                }
            }
        }
        ParamListKind::Type(t) => find_imports_expr(t, out),
        ParamListKind::Stream => {
            out.insert("/capnp/stream.capnp".to_string());
        }
    }
}

fn find_imports_decl(d: &Declaration, out: &mut BTreeSet<String>) {
    match &d.body {
        DeclBody::Using { target } => find_imports_expr(target, out),
        DeclBody::Const { ty, .. } => find_imports_expr(ty, out),
        DeclBody::Field { ty, .. } => find_imports_expr(ty, out),
        DeclBody::Interface { superclasses } => {
            superclasses.iter().for_each(|s| find_imports_expr(s, out))
        }
        DeclBody::Method { params, results } => {
            find_imports_params(params, out);
            if let Some(r) = results {
                find_imports_params(r, out);
            }
        }
        _ => {}
    }
    for a in &d.annotations {
        find_imports_expr(&a.name, out);
    }
    for n in &d.nested {
        find_imports_decl(n, out);
    }
}

/// Options for [`compile_files`].
#[derive(Clone, Debug, Default)]
pub struct CompileOptions {
    pub files: Vec<PathBuf>,
    pub src_prefixes: Vec<PathBuf>,
    pub import_paths: Vec<PathBuf>,
    pub no_standard_import: bool,
}

/// Everything needed to build a `CodeGeneratorRequest`.
pub struct CompileOutput {
    pub nodes: Vec<Rc<Node>>,
    pub source_info: Vec<SourceInfo>,
    pub requested_files: Vec<RequestedFileOut>,
}

pub struct RequestedFileOut {
    pub id: Id,
    pub filename: String,
    pub imports: Vec<(Id, String)>,
    pub resolutions: Vec<crate::source::Resolution>,
}

/// Compile the given files. On errors returns all diagnostics rendered with line/column.
pub fn compile_files(opts: &CompileOptions) -> Result<CompileOutput, Error> {
    let mut loader = SourceLoader::new();
    loader.import_paths = opts.import_paths.clone();
    loader.src_prefixes = opts.src_prefixes.clone();
    loader.no_standard_import = opts.no_standard_import;
    let mut c = Compiler::new(loader);

    for f in &opts.files {
        let (dir, path) = c.loader.interpret_source_file(f);
        let mut errs = vec![];
        let m = c.loader.load_module(&dir, &path, &mut errs);
        c.diagnostics.extend(errs);
        let Some(m) = m else {
            c.diagnostics.push(Diagnostic {
                file: f.display().to_string(),
                span: Span::default(),
                message: "no such file".into(),
            });
            continue;
        };
        let root = c.add_module(m);
        c.eagerly_compile(root, DEFAULT_EAGERNESS);
        let id = c.nodes[root].id;
        let filename = c.loader.modules[m].display_name.clone();
        c.requested.push(RequestedFile {
            id,
            filename,
            imports: vec![],
            module: m,
        });
    }

    if c.had_errors() {
        return Err(render_errors(&c));
    }

    let nodes = c.sim.all_loaded();
    let mut files = vec![];
    let reqs: Vec<(Id, String, usize)> = c
        .requested
        .iter()
        .map(|r| (r.id, r.filename.clone(), r.module))
        .collect();
    for (id, filename, module) in reqs {
        let imports = c.file_import_table(module);
        let resolutions = c.loader.modules[module].resolutions.clone();
        files.push(RequestedFileOut {
            id,
            filename,
            imports,
            resolutions,
        });
    }
    if c.had_errors() {
        return Err(render_errors(&c));
    }
    let source_info =
        crate::hashorder::order_source_info(std::mem::take(&mut c.source_info_inserted));
    Ok(CompileOutput {
        nodes,
        source_info,
        requested_files: files,
    })
}

fn render_errors(c: &Compiler) -> Error {
    let mut rendered = vec![];
    let mut seen = HashSet::new();
    for d in &c.diagnostics {
        let key = (d.file.clone(), d.span, d.message.clone());
        if !seen.insert(key) {
            continue;
        }
        let src = c
            .loader
            .modules
            .iter()
            .find(|m| m.display_name == d.file)
            .map(|m| m.source.as_slice());
        rendered.push(d.render(src));
    }
    Error {
        diagnostics: c.diagnostics.clone(),
        rendered,
    }
}

#[allow(dead_code)]
fn _unused(_: &Path, _: &Dir) {}
