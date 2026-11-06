//! Struct fields and enumerants.
//!
//! Pattern: Value Object — plain records describing members of struct and enum nodes.

use super::types::{Id, Type};
use super::values::{Annotation, Value};

/// How a field's ordinal was assigned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ordinal {
    /// Field has no ordinal (group or union).
    Implicit,
    /// Field has the given explicit ordinal.
    Explicit(u16),
}

/// Whether a field stores a value or opens a group.
#[derive(Clone, Debug, PartialEq)]
pub enum FieldKind {
    /// A field stored in the data or pointer section.
    Slot {
        /// Offset in units of the field's own size.
        offset: u32,
        /// Field type.
        ty: Type,
        /// Default value.
        default_value: Value,
        /// Whether the schema spelled out the default.
        had_explicit_default: bool,
    },
    /// A group of fields held in its own node.
    Group {
        /// Id of the group's struct node.
        type_id: Id,
    },
}

/// Discriminant value of a field that is not a union member.
pub const NO_DISCRIMINANT: u16 = 0xffff;

/// One field of a struct.
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    /// Field name.
    pub name: String,
    /// Position of the field in the source.
    pub code_order: u16,
    /// Annotations applied to the field.
    pub annotations: Vec<Annotation>,
    /// Union tag of the field, or [`NO_DISCRIMINANT`].
    pub discriminant_value: u16,
    /// Slot or group data.
    pub kind: FieldKind,
    /// Ordinal assignment.
    pub ordinal: Ordinal,
}

/// One member of an enum.
#[derive(Clone, Debug, PartialEq)]
pub struct Enumerant {
    /// Enumerant name.
    pub name: String,
    /// Position of the enumerant in the source.
    pub code_order: u16,
    /// Annotations applied to the enumerant.
    pub annotations: Vec<Annotation>,
}
