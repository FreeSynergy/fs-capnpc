//! Brand evaluation, type decompilation and brand compilation for a `BrandScope`.
//!
//! Pattern: Visitor — walks type expressions and schema types to bind or emit brands.

use crate::ast::*;
use crate::compiler::*;
use crate::error::Span;
use crate::generics::{BrandScope, BrandedDecl};
use crate::schema::{self, AnyPointer, Binding, Brand, BrandScopeKind, Type};
use std::rc::Rc;

impl BrandScope {
    /// Apply the type arguments of an application to a declaration, producing a branded scope.
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
        let parent = c
            .get_parent(decl.resolver)
            .map(|p| self.evaluate_brand(c, &p, brand, index));
        Rc::new(BrandScope {
            module: self.module,
            parent,
            leaf_id: decl.id,
            leaf_param_count: decl.generic_param_count,
            inherited,
            params,
        })
    }

    pub(super) fn builtin(self: &Rc<Self>, c: &mut Compiler, kind: DeclKind) -> BrandedDecl {
        let decl = c.resolve_builtin(kind);
        let brand = self.evaluate_brand(c, &decl, &[], 0);
        BrandedDecl {
            body: ResolveResult::Decl(decl),
            brand: Some(brand),
            source: None,
        }
    }

    /// Turn a schema `Type` back into a branded declaration.
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
            Type::Enum { type_id, brand }
            | Type::Struct { type_id, brand }
            | Type::Interface { type_id, brand } => {
                let decl = c.resolve_id(*type_id);
                let b = self.evaluate_brand(c, &decl, &brand.scopes, 0);
                BrandedDecl {
                    body: ResolveResult::Decl(decl),
                    brand: Some(b),
                    source: None,
                }
            }
            Type::List(el) => {
                let elem = self.decompile_type(c, el);
                let list = self.builtin(c, DeclKind::BuiltinList);
                list.apply_params(
                    c,
                    vec![elem],
                    &Expression {
                        kind: ExprKind::Unknown,
                        span: Span::default(),
                    },
                )
                .expect("List applies")
            }
            Type::AnyPointer(ap) => match ap {
                AnyPointer::Unconstrained(_) => self.builtin(c, DeclKind::BuiltinAnyPointer),
                AnyPointer::Parameter {
                    scope_id,
                    parameter_index,
                } => match self.lookup_parameter(c, *scope_id, *parameter_index as usize) {
                    Some(b) => b,
                    None => BrandedDecl {
                        body: ResolveResult::Param(ResolvedParameter {
                            id: *scope_id,
                            index: *parameter_index as usize,
                        }),
                        brand: None,
                        source: None,
                    },
                },
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
            scopes.push(schema::BrandScope {
                scope_id: lvl.leaf_id,
                kind,
            });
        }
        Brand { scopes }
    }
}
