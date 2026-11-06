//! `BrandedDecl`: a resolved declaration together with the brand scope it was resolved in.
//!
//! Pattern: Adapter — wraps a resolution result with its generic binding context.

use crate::ast::*;
use crate::compiler::*;
use crate::error::Span;
use crate::generics::BrandScope;
use crate::parser::expression_string;
use crate::schema::{Brand, Id};
use std::rc::Rc;

/// A resolved declaration together with its brand scope and source expression.
#[derive(Clone)]
pub struct BrandedDecl {
    /// The resolved declaration or parameter.
    pub body: ResolveResult,
    /// Generic bindings applied to the declaration.
    pub brand: Option<Rc<BrandScope>>,
    /// Expression the declaration came from, for error spans.
    pub source: Option<Rc<Expression>>,
}

fn err_span(source: Option<&Rc<Expression>>) -> Span {
    source.map(|s| s.span).unwrap_or_default()
}

impl std::fmt::Display for BrandedDecl {
    /// Source text of the expression this declaration came from.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match &self.source {
            Some(s) => expression_string(s),
            None => expression_string(&Expression {
                kind: ExprKind::Unknown,
                span: Span::default(),
            }),
        };
        f.write_str(&text)
    }
}

impl BrandedDecl {
    /// Branded declaration for implicit method generic parameter `index`.
    pub fn implicit_method_param(index: usize) -> BrandedDecl {
        BrandedDecl {
            body: ResolveResult::Param(ResolvedParameter { id: 0, index }),
            brand: None,
            source: None,
        }
    }

    /// Declaration kind, or `None` for a generic parameter.
    pub fn kind(&self) -> Option<DeclKind> {
        match &self.body {
            ResolveResult::Decl(d) => Some(d.kind),
            ResolveResult::Param(_) => None,
        }
    }

    /// Id of the generic declaration, or `None` for parameters.
    pub fn generic_type_id(&self) -> Option<Id> {
        match &self.body {
            ResolveResult::Decl(d) => Some(d.id),
            _ => None,
        }
    }

    /// Schema id of the declaration.
    pub fn id(&self) -> Id {
        match &self.body {
            ResolveResult::Decl(d) => d.id,
            _ => panic!("not a decl"),
        }
    }

    /// Record an error at `span` in `module`.
    pub fn add_error(&self, c: &mut Compiler, module: usize, msg: impl Into<String>) {
        c.add_error(module, err_span(self.source.as_ref()), msg);
    }

    /// Bind explicit type arguments to this declaration.
    pub fn apply_params(
        &self,
        c: &mut Compiler,
        params: Vec<BrandedDecl>,
        sub_source: &Expression,
    ) -> Option<BrandedDecl> {
        match &self.body {
            ResolveResult::Param(_) => None,
            ResolveResult::Decl(d) => {
                let scope = self
                    .brand
                    .as_ref()
                    .unwrap()
                    .set_params(c, params, d.kind, sub_source)?;
                Some(BrandedDecl {
                    body: self.body.clone(),
                    brand: Some(scope),
                    source: Some(Rc::new(sub_source.clone())),
                })
            }
        }
    }

    /// Resolve a member of this declaration, carrying the brand along.
    pub fn get_member(
        &self,
        c: &mut Compiler,
        name: &str,
        sub_source: &Expression,
    ) -> Option<BrandedDecl> {
        match &self.body {
            ResolveResult::Param(_) => None,
            ResolveResult::Decl(d) => {
                let r = c.resolve_member(d.resolver, name)?;
                Some(
                    self.brand
                        .as_ref()
                        .unwrap()
                        .interpret_resolve(c, &r, sub_source),
                )
            }
        }
    }

    /// Element type of a `List(T)` declaration.
    pub fn get_list_param(&self) -> Option<BrandedDecl> {
        let ResolveResult::Decl(d) = &self.body else {
            panic!("not a decl")
        };
        assert_eq!(d.kind, DeclKind::BuiltinList);
        let params = self.brand.as_ref().unwrap().get_params(d.id)?;
        if params.len() == 1 {
            Some(params[0].clone())
        } else {
            None
        }
    }

    /// Schema id and compiled brand of this declaration.
    pub fn id_and_brand(&self, c: &mut Compiler) -> (Id, Brand) {
        let brand = self.brand.as_ref().expect("decl has brand").compile(c);
        (self.id(), brand)
    }

    /// Convert back into a plain `ResolveResult`, recording the scope.
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
