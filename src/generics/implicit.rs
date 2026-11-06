//! Implicit generic parameters of a method's parameter and result structs.
//!
//! Pattern: Adapter — carries method-level parameter names into scope lookup.

use crate::schema::Id;

/// Implicit generic parameters of a method's parameter or result struct.
pub struct ImplicitParams {
    /// Id of the scope that owns the implicit parameters.
    pub scope_id: Id,
    /// Names of the implicit parameters.
    pub params: Vec<String>,
}

impl ImplicitParams {
    /// No implicit parameters.
    pub fn none() -> Self {
        ImplicitParams {
            scope_id: 0,
            params: vec![],
        }
    }
}
