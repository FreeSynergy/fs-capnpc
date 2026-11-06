//! Generic brands: the bindings applied to a generic declaration.
//!
//! Pattern: Composite — a brand holds scopes, which hold bindings, which may hold types again.

use super::types::{Id, Type};

/// A brand; an empty `scopes` list means "no brand" (the pointer is left null).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Brand {
    /// Bindings per generic scope.
    pub scopes: Vec<BrandScope>,
}

/// The bindings for one generic scope.
#[derive(Clone, Debug, PartialEq)]
pub struct BrandScope {
    /// Id of the declaration that owns the parameters.
    pub scope_id: Id,
    /// Whether the scope binds or inherits its parameters.
    pub kind: BrandScopeKind,
}

/// How a brand scope supplies its parameters.
#[derive(Clone, Debug, PartialEq)]
pub enum BrandScopeKind {
    /// Explicit binding per parameter.
    Bind(Vec<Binding>),
    /// Parameters are inherited from the enclosing scope.
    Inherit,
}

/// The binding of a single generic parameter.
#[derive(Clone, Debug, PartialEq)]
pub enum Binding {
    /// Parameter left unbound.
    Unbound,
    /// Parameter bound to a type.
    Type(Type),
}
