//! Schema types.
//!
//! Pattern: Composite — types nest through `List` and brand bindings.

use super::brand::Brand;

/// 64-bit unique id of a schema node.
pub type Id = u64;

/// A Cap'n Proto type.
#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    /// `Void`.
    Void,
    /// `Bool`.
    Bool,
    /// `Int8`.
    Int8,
    /// `Int16`.
    Int16,
    /// `Int32`.
    Int32,
    /// `Int64`.
    Int64,
    /// `UInt8`.
    UInt8,
    /// `UInt16`.
    UInt16,
    /// `UInt32`.
    UInt32,
    /// `UInt64`.
    UInt64,
    /// `Float32`.
    Float32,
    /// `Float64`.
    Float64,
    /// `Text`.
    Text,
    /// `Data`.
    Data,
    /// `List(T)` of the boxed element type.
    List(Box<Type>),
    /// Reference to an enum node.
    Enum {
        /// Id of the enum node.
        type_id: Id,
        /// Generic bindings of the enum.
        brand: Brand,
    },
    /// Reference to a struct node.
    Struct {
        /// Id of the struct node.
        type_id: Id,
        /// Generic bindings of the struct.
        brand: Brand,
    },
    /// Reference to an interface node.
    Interface {
        /// Id of the interface node.
        type_id: Id,
        /// Generic bindings of the interface.
        brand: Brand,
    },
    /// `AnyPointer` in one of its forms.
    AnyPointer(AnyPointer),
}

/// The form of an `AnyPointer` type.
#[derive(Clone, Debug, PartialEq)]
pub enum AnyPointer {
    /// Plain `AnyPointer`, `AnyStruct`, `AnyList` or `Capability`.
    Unconstrained(Unconstrained),
    /// A generic parameter of an enclosing declaration.
    Parameter {
        /// Id of the scope declaring the parameter.
        scope_id: Id,
        /// Position of the parameter in that scope.
        parameter_index: u16,
    },
    /// A generic parameter implicit to a method.
    ImplicitMethodParameter {
        /// Position among the method's implicit parameters.
        parameter_index: u16,
    },
}

/// Which kinds of pointer an unconstrained `AnyPointer` admits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unconstrained {
    /// Any pointer kind.
    AnyKind,
    /// Struct pointers only.
    Struct,
    /// List pointers only.
    List,
    /// Capability pointers only.
    Capability,
}
