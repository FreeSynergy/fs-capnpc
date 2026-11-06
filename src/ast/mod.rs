//! Parsed representation of a schema file (counterpart of `grammar.capnp`).
//!
//! Pattern: Composite — the syntax tree is split by concept; this module only re-exports the parts.

mod declarations;
mod expressions;
mod params;
mod targets;

pub use declarations::{BrandParameter, DeclBody, DeclId, Declaration};
pub use expressions::{AnnotationApplication, ExprKind, ExprParam, Expression, Located};
pub use params::{Param, ParamList, ParamListKind};
pub use targets::Targets;
