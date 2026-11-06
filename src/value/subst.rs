//! Type-parameter substitution through brands.
//!
//! Pattern: Visitor — recursively walks a `Type` replacing bound parameters.

use crate::schema::*;

/// Replace type parameters in `ty` by the bindings in `brand`.
pub(super) fn subst(ty: &Type, brand: &Brand) -> Type {
    match ty {
        Type::AnyPointer(AnyPointer::Parameter {
            scope_id,
            parameter_index,
        }) => {
            for s in &brand.scopes {
                if s.scope_id == *scope_id {
                    if let BrandScopeKind::Bind(b) = &s.kind {
                        return match b.get(*parameter_index as usize) {
                            Some(Binding::Type(t)) => t.clone(),
                            Some(Binding::Unbound) | None => {
                                Type::AnyPointer(AnyPointer::Unconstrained(Unconstrained::AnyKind))
                            }
                        };
                    }
                }
            }
            ty.clone()
        }
        Type::List(e) => Type::List(Box::new(subst(e, brand))),
        Type::Struct { type_id, brand: b } => Type::Struct {
            type_id: *type_id,
            brand: subst_brand(b, brand),
        },
        Type::Enum { type_id, brand: b } => Type::Enum {
            type_id: *type_id,
            brand: subst_brand(b, brand),
        },
        Type::Interface { type_id, brand: b } => Type::Interface {
            type_id: *type_id,
            brand: subst_brand(b, brand),
        },
        _ => ty.clone(),
    }
}

pub(super) fn subst_brand(b: &Brand, outer: &Brand) -> Brand {
    Brand {
        scopes: b
            .scopes
            .iter()
            .map(|s| BrandScope {
                scope_id: s.scope_id,
                kind: match &s.kind {
                    BrandScopeKind::Bind(v) => BrandScopeKind::Bind(
                        v.iter()
                            .map(|x| match x {
                                Binding::Type(t) => Binding::Type(subst(t, outer)),
                                Binding::Unbound => Binding::Unbound,
                            })
                            .collect(),
                    ),
                    BrandScopeKind::Inherit => BrandScopeKind::Inherit,
                },
            })
            .collect(),
    }
}
