//! Generic parameter bookkeeping (counterpart of `generics.c++`): `BrandScope` tracks which type
//! arguments are bound at each level of the declaration nesting, `BrandedDecl` is a resolved
//! declaration together with its brand.

use crate::ast::*;
use crate::compiler::*;
use crate::error::Span;
use crate::parser::expression_string;
use crate::schema::{self, AnyPointer, Binding, Brand, BrandScopeKind, Id, Type, Unconstrained};
use std::rc::Rc;

pub struct ImplicitParams {
    pub scope_id: Id,
    pub params: Vec<String>,
}

impl ImplicitParams {
    pub fn none() -> Self {
        ImplicitParams { scope_id: 0, params: vec![] }
    }
}

#[derive(Clone)]
pub struct BrandedDecl {
    pub body: ResolveResult,
    pub brand: Option<Rc<BrandScope>>,
    pub source: Option<Rc<Expression>>,
}

pub struct BrandScope {
    module: usize,
    parent: Option<Rc<BrandScope>>,
    leaf_id: Id,
    leaf_param_count: usize,
    inherited: bool,
    params: Vec<BrandedDecl>,
}

fn err_span(source: &Option<Rc<Expression>>) -> Span {
    source.as_ref().map(|s| s.span).unwrap_or_default()
}

impl BrandedDecl {
    pub fn implicit_method_param(index: usize) -> BrandedDecl {
        BrandedDecl { body: ResolveResult::Param(ResolvedParameter { id: 0, index }), brand: None, source: None }
    }

    pub fn kind(&self) -> Option<DeclKind> {
        match &self.body {
            ResolveResult::Decl(d) => Some(d.kind),
            ResolveResult::Param(_) => None,
        }
    }

    pub fn generic_type_id(&self) -> Option<Id> {
        match &self.body {
            ResolveResult::Decl(d) => Some(d.id),
            _ => None,
        }
    }

    pub fn id(&self) -> Id {
        match &self.body {
            ResolveResult::Decl(d) => d.id,
            _ => panic!("not a decl"),
        }
    }

    pub fn add_error(&self, c: &mut Compiler, module: usize, msg: impl Into<String>) {
        c.add_error(module, err_span(&self.source), msg);
    }

    pub fn to_string(&self) -> String {
        match &self.source {
            Some(s) => expression_string(s),
            None => expression_string(&Expression { kind: ExprKind::Unknown, span: Span::default() }),
        }
    }

    pub fn apply_params(
        &self,
        c: &mut Compiler,
        params: Vec<BrandedDecl>,
        sub_source: &Expression,
    ) -> Option<BrandedDecl> {
        match &self.body {
            ResolveResult::Param(_) => None,
            ResolveResult::Decl(d) => {
                let scope = self.brand.as_ref().unwrap().set_params(c, params, d.kind, sub_source)?;
                Some(BrandedDecl {
                    body: self.body.clone(),
                    brand: Some(scope),
                    source: Some(Rc::new(sub_source.clone())),
                })
            }
        }
    }

    pub fn get_member(&self, c: &mut Compiler, name: &str, sub_source: &Expression) -> Option<BrandedDecl> {
        match &self.body {
            ResolveResult::Param(_) => None,
            ResolveResult::Decl(d) => {
                let r = c.resolve_member(d.resolver, name)?;
                Some(self.brand.as_ref().unwrap().interpret_resolve(c, &r, sub_source))
            }
        }
    }

    pub fn get_list_param(&self) -> Option<BrandedDecl> {
        let d = match &self.body {
            ResolveResult::Decl(d) => d,
            _ => panic!("not a decl"),
        };
        assert_eq!(d.kind, DeclKind::BuiltinList);
        let params = self.brand.as_ref().unwrap().get_params(d.id)?;
        if params.len() != 1 { None } else { Some(params[0].clone()) }
    }

    pub fn id_and_brand(&self, c: &mut Compiler) -> (Id, Brand) {
        let brand = self.brand.as_ref().expect("decl has brand").compile(c);
        (self.id(), brand)
    }

    /// `compileAsType`: returns the schema type, or `None` after reporting an error.
    pub fn compile_as_type(&self, c: &mut Compiler, module: usize) -> Option<Type> {
        match self.kind() {
            Some(kind) => match kind {
                DeclKind::Enum => {
                    let (type_id, brand) = self.id_and_brand(c);
                    Some(Type::Enum { type_id, brand })
                }
                DeclKind::Struct => {
                    let (type_id, brand) = self.id_and_brand(c);
                    Some(Type::Struct { type_id, brand })
                }
                DeclKind::Interface => {
                    let (type_id, brand) = self.id_and_brand(c);
                    Some(Type::Interface { type_id, brand })
                }
                DeclKind::BuiltinList => {
                    let Some(param) = self.get_list_param() else {
                        self.add_error(c, module, "'List' requires exactly one parameter.");
                        return None;
                    };
                    let elem = param.compile_as_type(c, module)?;
                    if let Type::AnyPointer(AnyPointer::Unconstrained(u)) = &elem {
                        match u {
                            Unconstrained::AnyKind => {
                                self.add_error(c, module, "'List(AnyPointer)' is not supported.");
                                return None;
                            }
                            Unconstrained::Struct => {
                                self.add_error(c, module, "'List(AnyStruct)' is not supported.");
                                return None;
                            }
                            _ => {}
                        }
                    }
                    Some(Type::List(Box::new(elem)))
                }
                DeclKind::BuiltinVoid => Some(Type::Void),
                DeclKind::BuiltinBool => Some(Type::Bool),
                DeclKind::BuiltinInt8 => Some(Type::Int8),
                DeclKind::BuiltinInt16 => Some(Type::Int16),
                DeclKind::BuiltinInt32 => Some(Type::Int32),
                DeclKind::BuiltinInt64 => Some(Type::Int64),
                DeclKind::BuiltinUInt8 => Some(Type::UInt8),
                DeclKind::BuiltinUInt16 => Some(Type::UInt16),
                DeclKind::BuiltinUInt32 => Some(Type::UInt32),
                DeclKind::BuiltinUInt64 => Some(Type::UInt64),
                DeclKind::BuiltinFloat32 => Some(Type::Float32),
                DeclKind::BuiltinFloat64 => Some(Type::Float64),
                DeclKind::BuiltinText => Some(Type::Text),
                DeclKind::BuiltinData => Some(Type::Data),
                DeclKind::BuiltinObject => {
                    self.add_error(c, module, "As of Cap'n Proto 0.4, 'Object' has been renamed to 'AnyPointer'.  Sorry for the inconvenience, and thanks for being an early adopter.  :)");
                    Some(Type::AnyPointer(AnyPointer::Unconstrained(Unconstrained::AnyKind)))
                }
                DeclKind::BuiltinAnyPointer => Some(Type::AnyPointer(AnyPointer::Unconstrained(Unconstrained::AnyKind))),
                DeclKind::BuiltinAnyStruct => Some(Type::AnyPointer(AnyPointer::Unconstrained(Unconstrained::Struct))),
                DeclKind::BuiltinAnyList => Some(Type::AnyPointer(AnyPointer::Unconstrained(Unconstrained::List))),
                DeclKind::BuiltinCapability => {
                    Some(Type::AnyPointer(AnyPointer::Unconstrained(Unconstrained::Capability)))
                }
                _ => {
                    let msg = format!("'{}' is not a type.", self.to_string());
                    self.add_error(c, module, msg);
                    None
                }
            },
            None => {
                let ResolveResult::Param(var) = &self.body else { unreachable!() };
                if var.id == 0 {
                    Some(Type::AnyPointer(AnyPointer::ImplicitMethodParameter { parameter_index: var.index as u16 }))
                } else {
                    Some(Type::AnyPointer(AnyPointer::Parameter {
                        scope_id: var.id,
                        parameter_index: var.index as u16,
                    }))
                }
            }
        }
    }

    pub fn as_resolve_result(&self, c: &mut Compiler, scope_id: Id) -> ResolveResult {
        let mut result = self.body.clone();
        if let ResolveResult::Decl(d) = &mut result {
            d.scope_id = scope_id;
            let b = self.brand.as_ref().expect("brand").compile(c);
            if !b.scopes.is_empty() {
                d.brand = Some(b);
            }
        }
        result
    }
}

impl BrandScope {
    pub fn new_root(c: &mut Compiler, module: usize, id: Id, param_count: usize, resolver: NodeIdx) -> Rc<BrandScope> {
        let parent = c
            .get_parent(resolver)
            .map(|p| BrandScope::new_root(c, module, p.id, p.generic_param_count, p.resolver));
        Rc::new(BrandScope { module, parent, leaf_id: id, leaf_param_count: param_count, inherited: true, params: vec![] })
    }

    fn simple(module: usize, scope_id: Id) -> Rc<BrandScope> {
        Rc::new(BrandScope {
            module,
            parent: None,
            leaf_id: scope_id,
            leaf_param_count: 0,
            inherited: false,
            params: vec![],
        })
    }

    pub fn leaf_id(&self) -> Id {
        self.leaf_id
    }

    pub fn is_generic(&self) -> bool {
        self.leaf_param_count > 0 || self.parent.as_ref().is_some_and(|p| p.is_generic())
    }

    pub fn push(self: &Rc<Self>, type_id: Id, param_count: usize) -> Rc<BrandScope> {
        Rc::new(BrandScope {
            module: self.module,
            parent: Some(self.clone()),
            leaf_id: type_id,
            leaf_param_count: param_count,
            inherited: false,
            params: vec![],
        })
    }

    pub fn set_params(
        self: &Rc<Self>,
        c: &mut Compiler,
        params: Vec<BrandedDecl>,
        generic_type: DeclKind,
        source: &Expression,
    ) -> Option<Rc<BrandScope>> {
        if !self.params.is_empty() {
            c.add_error(self.module, source.span, "Double-application of generic parameters.");
            None
        } else if params.len() > self.leaf_param_count {
            if self.leaf_param_count == 0 {
                c.add_error(self.module, source.span, "Declaration does not accept generic parameters.");
            } else {
                c.add_error(self.module, source.span, "Too many generic parameters.");
            }
            None
        } else if params.len() < self.leaf_param_count {
            c.add_error(self.module, source.span, "Not enough generic parameters.");
            None
        } else {
            if generic_type != DeclKind::BuiltinList {
                for p in &params {
                    if let Some(kind) = p.kind() {
                        match kind {
                            DeclKind::BuiltinList
                            | DeclKind::BuiltinText
                            | DeclKind::BuiltinData
                            | DeclKind::BuiltinAnyPointer
                            | DeclKind::Struct
                            | DeclKind::Interface => {}
                            _ => p.add_error(c, self.module, "Sorry, only pointer types can be used as generic parameters."),
                        }
                    }
                }
            }
            Some(Rc::new(BrandScope {
                module: self.module,
                parent: self.parent.clone(),
                leaf_id: self.leaf_id,
                leaf_param_count: self.leaf_param_count,
                inherited: false,
                params,
            }))
        }
    }

    pub fn pop(self: &Rc<Self>, new_leaf_id: Id) -> Rc<BrandScope> {
        if self.leaf_id == new_leaf_id {
            return self.clone();
        }
        match &self.parent {
            Some(p) => p.pop(new_leaf_id),
            None => BrandScope::simple(self.module, new_leaf_id),
        }
    }

    pub fn lookup_parameter(self: &Rc<Self>, c: &mut Compiler, scope_id: Id, index: usize) -> Option<BrandedDecl> {
        if scope_id == self.leaf_id {
            if index < self.params.len() {
                Some(self.params[index].clone())
            } else if self.inherited {
                None
            } else {
                let decl = c.resolve_builtin(DeclKind::BuiltinAnyPointer);
                let brand = self.evaluate_brand(c, &decl, &[], 0);
                Some(BrandedDecl { body: ResolveResult::Decl(decl), brand: Some(brand), source: None })
            }
        } else {
            self.parent.as_ref().and_then(|p| p.lookup_parameter(c, scope_id, index))
        }
    }

    /// `None` means "inherited from the client scope".
    pub fn get_params(&self, scope_id: Id) -> Option<Vec<BrandedDecl>> {
        if scope_id == self.leaf_id {
            if self.inherited { None } else { Some(self.params.clone()) }
        } else {
            self.parent.as_ref().and_then(|p| p.get_params(scope_id))
        }
    }

    pub fn interpret_resolve(self: &Rc<Self>, c: &mut Compiler, result: &ResolveResult, source: &Expression) -> BrandedDecl {
        match result {
            ResolveResult::Decl(decl) => {
                let mut scope = self.pop(decl.scope_id);
                if let Some(brand) = &decl.brand {
                    scope = scope.evaluate_brand(c, decl, &brand.scopes, 0);
                } else {
                    scope = scope.push(decl.id, decl.generic_param_count);
                }
                BrandedDecl { body: result.clone(), brand: Some(scope), source: Some(Rc::new(source.clone())) }
            }
            ResolveResult::Param(p) => match self.lookup_parameter(c, p.id, p.index) {
                Some(d) => d,
                None => BrandedDecl { body: result.clone(), brand: None, source: Some(Rc::new(source.clone())) },
            },
        }
    }

    pub fn evaluate_brand(
        self: &Rc<Self>,
        c: &mut Compiler,
        decl: &ResolvedDecl,
        brand: &[schema::BrandScope],
        index: usize,
    ) -> Rc<BrandScope> {
        let mut params = vec![];
        let mut inherited = false;
        let mut index = index;
        if index < brand.len() {
            let next = &brand[index];
            if decl.id == next.scope_id {
                match &next.kind {
                    BrandScopeKind::Bind(bindings) => {
                        for b in bindings {
                            match b {
                                Binding::Unbound => {
                                    let ap = c.resolve_builtin(DeclKind::BuiltinAnyPointer);
                                    let scope = BrandScope::simple(self.module, ap.scope_id);
                                    params.push(BrandedDecl {
                                        body: ResolveResult::Decl(ap),
                                        brand: Some(scope),
                                        source: None,
                                    });
                                }
                                Binding::Type(t) => params.push(self.decompile_type(c, t)),
                            }
                        }
                    }
                    BrandScopeKind::Inherit => match self.get_params(decl.id) {
                        Some(p) => params = p,
                        None => inherited = true,
                    },
                }
                index += 1;
            }
        }
        let parent = c.get_parent(decl.resolver).map(|p| self.evaluate_brand(c, &p, brand, index));
        Rc::new(BrandScope {
            module: self.module,
            parent,
            leaf_id: decl.id,
            leaf_param_count: decl.generic_param_count,
            inherited,
            params,
        })
    }

    fn builtin(self: &Rc<Self>, c: &mut Compiler, kind: DeclKind) -> BrandedDecl {
        let decl = c.resolve_builtin(kind);
        let brand = self.evaluate_brand(c, &decl, &[], 0);
        BrandedDecl { body: ResolveResult::Decl(decl), brand: Some(brand), source: None }
    }

    pub fn decompile_type(self: &Rc<Self>, c: &mut Compiler, ty: &Type) -> BrandedDecl {
        match ty {
            Type::Void => self.builtin(c, DeclKind::BuiltinVoid),
            Type::Bool => self.builtin(c, DeclKind::BuiltinBool),
            Type::Int8 => self.builtin(c, DeclKind::BuiltinInt8),
            Type::Int16 => self.builtin(c, DeclKind::BuiltinInt16),
            Type::Int32 => self.builtin(c, DeclKind::BuiltinInt32),
            Type::Int64 => self.builtin(c, DeclKind::BuiltinInt64),
            Type::UInt8 => self.builtin(c, DeclKind::BuiltinUInt8),
            Type::UInt16 => self.builtin(c, DeclKind::BuiltinUInt16),
            Type::UInt32 => self.builtin(c, DeclKind::BuiltinUInt32),
            Type::UInt64 => self.builtin(c, DeclKind::BuiltinUInt64),
            Type::Float32 => self.builtin(c, DeclKind::BuiltinFloat32),
            Type::Float64 => self.builtin(c, DeclKind::BuiltinFloat64),
            Type::Text => self.builtin(c, DeclKind::BuiltinText),
            Type::Data => self.builtin(c, DeclKind::BuiltinData),
            Type::Enum { type_id, brand } | Type::Struct { type_id, brand } | Type::Interface { type_id, brand } => {
                let decl = c.resolve_id(*type_id);
                let b = self.evaluate_brand(c, &decl, &brand.scopes, 0);
                BrandedDecl { body: ResolveResult::Decl(decl), brand: Some(b), source: None }
            }
            Type::List(el) => {
                let elem = self.decompile_type(c, el);
                let list = self.builtin(c, DeclKind::BuiltinList);
                list.apply_params(c, vec![elem], &Expression { kind: ExprKind::Unknown, span: Span::default() })
                    .expect("List applies")
            }
            Type::AnyPointer(ap) => match ap {
                AnyPointer::Unconstrained(_) => self.builtin(c, DeclKind::BuiltinAnyPointer),
                AnyPointer::Parameter { scope_id, parameter_index } => {
                    match self.lookup_parameter(c, *scope_id, *parameter_index as usize) {
                        Some(b) => b,
                        None => BrandedDecl {
                            body: ResolveResult::Param(ResolvedParameter {
                                id: *scope_id,
                                index: *parameter_index as usize,
                            }),
                            brand: None,
                            source: None,
                        },
                    }
                }
                AnyPointer::ImplicitMethodParameter { .. } => {
                    panic!("Alias pointed to implicit method type parameter?")
                }
            },
        }
    }

    /// `BrandScope::compile`: the schema brand describing this scope (empty if nothing is bound).
    pub fn compile(self: &Rc<Self>, c: &mut Compiler) -> Brand {
        let mut levels: Vec<&BrandScope> = vec![];
        let mut ptr: &BrandScope = self;
        loop {
            if !ptr.params.is_empty() || (ptr.inherited && ptr.leaf_param_count > 0) {
                levels.push(ptr);
            }
            match &ptr.parent {
                Some(p) => ptr = p,
                None => break,
            }
        }
        let mut scopes = vec![];
        for lvl in levels {
            let kind = if lvl.inherited {
                BrandScopeKind::Inherit
            } else {
                let mut b = vec![];
                for p in &lvl.params {
                    let t = p.compile_as_type(c, self.module).unwrap_or(Type::Void);
                    b.push(Binding::Type(t));
                }
                BrandScopeKind::Bind(b)
            };
            scopes.push(schema::BrandScope { scope_id: lvl.leaf_id, kind });
        }
        Brand { scopes }
    }

    pub fn compile_decl_expression(
        self: &Rc<Self>,
        c: &mut Compiler,
        source: &Expression,
        resolver: NodeIdx,
        implicit: &ImplicitParams,
    ) -> Option<BrandedDecl> {
        match &source.kind {
            ExprKind::Unknown => None,
            ExprKind::PositiveInt(_)
            | ExprKind::NegativeInt(_)
            | ExprKind::Float(_)
            | ExprKind::String(_)
            | ExprKind::Binary(_)
            | ExprKind::List(_)
            | ExprKind::Tuple(_)
            | ExprKind::Embed(_) => {
                c.add_error(self.module, source.span, "Expected name.");
                None
            }
            ExprKind::RelativeName(name) => {
                for (i, p) in implicit.params.iter().enumerate() {
                    if p == name {
                        return Some(if implicit.scope_id == 0 {
                            BrandedDecl::implicit_method_param(i)
                        } else {
                            BrandedDecl {
                                body: ResolveResult::Param(ResolvedParameter { id: implicit.scope_id, index: i }),
                                brand: None,
                                source: None,
                            }
                        });
                    }
                }
                match c.resolve(resolver, name) {
                    Some(r) => {
                        let result = self.interpret_resolve(c, &r, source);
                        if let ResolveResult::Decl(d) = &r {
                            c.report_resolution(self.module, source.span.start, source.span.end, d.id);
                        }
                        Some(result)
                    }
                    None => {
                        c.add_error(self.module, source.span, format!("Not defined: {name}"));
                        None
                    }
                }
            }
            ExprKind::AbsoluteName(name) => {
                let top = c.get_top_scope(resolver);
                match c.resolve_member(top.resolver, name) {
                    Some(r) => {
                        let result = self.interpret_resolve(c, &r, source);
                        if let ResolveResult::Decl(d) = &r {
                            c.report_resolution(self.module, source.span.start, source.span.end, d.id);
                        }
                        Some(result)
                    }
                    None => {
                        c.add_error(self.module, source.span, format!("Not defined: {name}"));
                        None
                    }
                }
            }
            ExprKind::Import(filename) => match c.resolve_import(resolver, filename) {
                Some(decl) => {
                    c.report_resolution(self.module, source.span.start, source.span.end, decl.id);
                    let scope = BrandScope::new_root(c, self.module, decl.id, decl.generic_param_count, decl.resolver);
                    Some(BrandedDecl {
                        body: ResolveResult::Decl(decl),
                        brand: Some(scope),
                        source: Some(Rc::new(source.clone())),
                    })
                }
                None => {
                    c.add_error(self.module, source.span, format!("Import failed: {filename}"));
                    None
                }
            },
            ExprKind::Application { function, params } => {
                let decl = self.compile_decl_expression(c, function, resolver, implicit)?;
                let mut compiled = vec![];
                let mut failed = false;
                for p in params {
                    if let Some(n) = &p.name {
                        c.add_error(self.module, n.span, "Named parameter not allowed here.");
                    }
                    match self.compile_decl_expression(c, &p.value, resolver, implicit) {
                        Some(d) => compiled.push(d),
                        None => failed = true,
                    }
                }
                if failed {
                    return Some(decl);
                }
                match decl.apply_params(c, compiled, source) {
                    Some(a) => Some(a),
                    None => Some(decl),
                }
            }
            ExprKind::Member { parent, name } => {
                let decl = self.compile_decl_expression(c, parent, resolver, implicit)?;
                match decl.get_member(c, &name.value, source) {
                    Some(m) => {
                        if let Some(id) = m.generic_type_id() {
                            c.report_resolution(self.module, source.span.start, source.span.end, id);
                        }
                        Some(m)
                    }
                    None => {
                        c.add_error(
                            self.module,
                            name.span,
                            format!("'{}' has no member named '{}'", expression_string(parent), name.value),
                        );
                        None
                    }
                }
            }
        }
    }
}
