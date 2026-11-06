//! Declaration kinds, built-in type table and traversal eagerness flags.
//!
//! Pattern: Registry — one table of built-in declarations keyed by name and kind.

use crate::ast::*;

// Eagerness flags for traversal.
/// Eagerness flag: compile the node itself.
pub const NODE: u32 = 1;
/// Eagerness flag: also visit parent nodes.
pub const PARENTS: u32 = 2;
/// Eagerness flag: also visit nested nodes.
pub const CHILDREN: u32 = 4;
/// Eagerness shift: flags above this apply to dependencies.
pub const DEPENDENCIES: u32 = 1 << 15;
/// Eagerness flag: visit the parents of dependencies.
pub const DEPENDENCY_PARENTS: u32 = PARENTS * DEPENDENCIES;
/// Eagerness used for requested files.
pub const DEFAULT_EAGERNESS: u32 = NODE | CHILDREN | DEPENDENCIES | DEPENDENCY_PARENTS;

/// Kind of a declaration, including the built-in types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DeclKind {
    /// A file (module root).
    File,
    /// A `using` alias.
    Using,
    /// A constant.
    Const,
    /// An enum.
    Enum,
    /// An enum value.
    Enumerant,
    /// A struct.
    Struct,
    /// A struct field.
    Field,
    /// A union.
    Union,
    /// A group.
    Group,
    /// An interface.
    Interface,
    /// An interface method.
    Method,
    /// An annotation declaration.
    Annotation,
    /// Built-in type `Void`.
    BuiltinVoid,
    /// Built-in type `Bool`.
    BuiltinBool,
    /// Built-in type `Int8`.
    BuiltinInt8,
    /// Built-in type `Int16`.
    BuiltinInt16,
    /// Built-in type `Int32`.
    BuiltinInt32,
    /// Built-in type `Int64`.
    BuiltinInt64,
    /// Built-in type `UInt8`.
    BuiltinUInt8,
    /// Built-in type `UInt16`.
    BuiltinUInt16,
    /// Built-in type `UInt32`.
    BuiltinUInt32,
    /// Built-in type `UInt64`.
    BuiltinUInt64,
    /// Built-in type `Float32`.
    BuiltinFloat32,
    /// Built-in type `Float64`.
    BuiltinFloat64,
    /// Built-in type `Text`.
    BuiltinText,
    /// Built-in type `Data`.
    BuiltinData,
    /// Built-in type `List`.
    BuiltinList,
    /// Built-in type `Object`.
    BuiltinObject,
    /// Built-in type `AnyPointer`.
    BuiltinAnyPointer,
    /// Built-in type `AnyStruct`.
    BuiltinAnyStruct,
    /// Built-in type `AnyList`.
    BuiltinAnyList,
    /// Built-in type `Capability`.
    BuiltinCapability,
}

/// Map a parsed declaration to its `DeclKind`.
pub fn decl_kind(d: &Declaration) -> DeclKind {
    match &d.body {
        DeclBody::File => DeclKind::File,
        DeclBody::Using { .. } => DeclKind::Using,
        DeclBody::Const { .. } => DeclKind::Const,
        DeclBody::Enum => DeclKind::Enum,
        DeclBody::Enumerant => DeclKind::Enumerant,
        DeclBody::Struct => DeclKind::Struct,
        DeclBody::Field { .. } => DeclKind::Field,
        DeclBody::Union => DeclKind::Union,
        DeclBody::Group => DeclKind::Group,
        DeclBody::Interface { .. } => DeclKind::Interface,
        DeclBody::Method { .. } => DeclKind::Method,
        DeclBody::Annotation { .. } => DeclKind::Annotation,
    }
}

pub(super) const BUILTINS: &[(&str, DeclKind, usize)] = &[
    ("Void", DeclKind::BuiltinVoid, 0),
    ("Bool", DeclKind::BuiltinBool, 0),
    ("Int8", DeclKind::BuiltinInt8, 0),
    ("Int16", DeclKind::BuiltinInt16, 0),
    ("Int32", DeclKind::BuiltinInt32, 0),
    ("Int64", DeclKind::BuiltinInt64, 0),
    ("UInt8", DeclKind::BuiltinUInt8, 0),
    ("UInt16", DeclKind::BuiltinUInt16, 0),
    ("UInt32", DeclKind::BuiltinUInt32, 0),
    ("UInt64", DeclKind::BuiltinUInt64, 0),
    ("Float32", DeclKind::BuiltinFloat32, 0),
    ("Float64", DeclKind::BuiltinFloat64, 0),
    ("Text", DeclKind::BuiltinText, 0),
    ("Data", DeclKind::BuiltinData, 0),
    ("List", DeclKind::BuiltinList, 1),
    ("Object", DeclKind::BuiltinObject, 0),
    ("AnyPointer", DeclKind::BuiltinAnyPointer, 0),
    ("AnyStruct", DeclKind::BuiltinAnyStruct, 0),
    ("AnyList", DeclKind::BuiltinAnyList, 0),
    ("Capability", DeclKind::BuiltinCapability, 0),
];
