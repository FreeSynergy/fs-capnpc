//! Parameter lookup and expression interpretation inside a `BrandScope`.
//!
//! Pattern: Strategy — resolves a name result differently for declarations and parameters.

use crate::ast::*;
use crate::compiler::*;
use crate::generics::{BrandScope, BrandedDecl};
use crate::schema::Id;
use std::rc::Rc;

impl BrandScope {
    /// Find the binding of generic parameter `index` of scope `scope_id`.
    pub fn lookup_parameter(
        self: &Rc<Self>,
        c: &mut Compiler,
        scope_id: Id,
        index: usize,
    ) -> Option<BrandedDecl> {
        if scope_id == self.leaf_id {
            if index < self.params.len() {
                Some(self.params[index].clone())
            } else if self.inherited {
                None
            } else {
                let decl = c.resolve_builtin(DeclKind::BuiltinAnyPointer);
                let brand = self.evaluate_brand(c, &decl, &[], 0);
                Some(BrandedDecl {
                    body: ResolveResult::Decl(decl),
                    brand: Some(brand),
                    source: None,
                })
            }
        } else {
            self.parent
                .as_ref()
                .and_then(|p| p.lookup_parameter(c, scope_id, index))
        }
    }

    /// `None` means "inherited from the client scope".
    pub fn get_params(&self, scope_id: Id) -> Option<Vec<BrandedDecl>> {
        if scope_id == self.leaf_id {
            if self.inherited {
                None
            } else {
                Some(self.params.clone())
            }
        } else {
            self.parent.as_ref().and_then(|p| p.get_params(scope_id))
        }
    }

    /// Combine a resolution result with this scope into a branded declaration.
    pub fn interpret_resolve(
        self: &Rc<Self>,
        c: &mut Compiler,
        result: &ResolveResult,
        source: &Expression,
    ) -> BrandedDecl {
        match result {
            ResolveResult::Decl(decl) => {
                let mut scope = self.pop(decl.scope_id);
                if let Some(brand) = &decl.brand {
                    scope = scope.evaluate_brand(c, decl, &brand.scopes, 0);
                } else {
                    scope = scope.push(decl.id, decl.generic_param_count);
                }
                BrandedDecl {
                    body: result.clone(),
                    brand: Some(scope),
                    source: Some(Rc::new(source.clone())),
                }
            }
            ResolveResult::Param(p) => match self.lookup_parameter(c, p.id, p.index) {
                Some(d) => d,
                None => BrandedDecl {
                    body: result.clone(),
                    brand: None,
                    source: Some(Rc::new(source.clone())),
                },
            },
        }
    }
}
