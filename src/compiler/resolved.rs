//! Results of name resolution: resolved declarations and generic parameters.
//!
//! Pattern: Adapter — presents compiler nodes as plain value types for the generics layer.

use crate::compiler::kinds::DeclKind;
use crate::schema::{Brand, Id};

/// A declaration found by name resolution.
#[derive(Clone, Debug)]
pub struct ResolvedDecl {
    /// Schema id.
    pub id: Id,
    /// Number of generic parameters.
    pub generic_param_count: usize,
    /// Id of the scope the declaration was found in.
    pub scope_id: Id,
    /// Declaration kind.
    pub kind: DeclKind,
    /// Node index used to resolve names inside the declaration.
    pub resolver: usize,
    /// Brand attached during application, if any.
    pub brand: Option<Brand>,
}

/// A generic parameter found by name resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedParameter {
    /// Schema id.
    pub id: Id,
    /// Parameter position.
    pub index: usize,
}

/// Result of name resolution: a declaration or a generic parameter.
#[derive(Clone, Debug)]
pub enum ResolveResult {
    /// A declaration.
    Decl(ResolvedDecl),
    /// A generic parameter.
    Param(ResolvedParameter),
}
