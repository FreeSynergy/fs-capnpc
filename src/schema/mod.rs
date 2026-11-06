//! In-memory form of the schema nodes the compiler produces (mirrors `schema.capnp`).
//!
//! Pattern: Composite — the node tree is split by concept; this module only re-exports the parts.
//!
//! The translator builds these plain structs; [`crate::request`] serialises them with the
//! `schema_capnp` builders. Where the official compiler distinguishes an absent list pointer from an
//! empty one (observable in the encoded message), the corresponding field here is an `Option`.

mod brand;
mod field;
mod node;
mod source_info;
mod types;
mod values;

pub use crate::ast::Targets;
pub use brand::{Binding, Brand, BrandScope, BrandScopeKind};
pub use field::{Enumerant, Field, FieldKind, NO_DISCRIMINANT, Ordinal};
pub use node::{InterfaceNode, Method, Node, NodeKind, NodeSet, StructNode, Superclass};
pub use source_info::{MemberSourceInfo, SourceInfo};
pub use types::{AnyPointer, Id, Type, Unconstrained};
pub use values::{Annotation, ListVal, Ptr, StructVal, Value};
