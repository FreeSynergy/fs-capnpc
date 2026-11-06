//! Constant values, pointer payload trees and annotation uses.
//!
//! Pattern: Composite — pointer values nest struct and list payloads recursively.

use super::brand::Brand;
use super::types::Id;

/// A pointer-typed value tree (the payload of `Value.text`, `.data`, `.list`, `.struct`, ...).
#[derive(Clone, Debug, PartialEq)]
pub enum Ptr {
    /// Null pointer.
    Null,
    /// Text bytes without the terminator.
    Text(Vec<u8>),
    /// Raw data bytes.
    Data(Vec<u8>),
    /// A struct payload.
    Struct(StructVal),
    /// A list payload.
    List(ListVal),
}

/// A struct payload in wire layout.
#[derive(Clone, Debug, PartialEq)]
pub struct StructVal {
    /// Size of the data section in words.
    pub data_words: u16,
    /// Number of pointer slots.
    pub pointer_count: u16,
    /// `data_words * 8` bytes.
    pub data: Vec<u8>,
    /// Pointer section contents.
    pub ptrs: Vec<Ptr>,
}

/// A list payload in wire layout.
#[derive(Clone, Debug, PartialEq)]
pub enum ListVal {
    /// List of void elements with the given length.
    Void(u32),
    /// List of single-bit elements.
    Bits(Vec<bool>),
    /// Element size in bytes (1, 2, 4 or 8) and little-endian values.
    Bytes {
        /// Element size in bytes.
        size: u8,
        /// Element values widened to 64 bits.
        values: Vec<u64>,
    },
    /// List of pointers.
    Pointers(Vec<Ptr>),
    /// Composite list of structs.
    Structs {
        /// Data words per element.
        data_words: u16,
        /// Pointer slots per element.
        pointer_count: u16,
        /// The elements.
        elems: Vec<StructVal>,
    },
}

/// A constant value of some schema type.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// `Void`.
    Void,
    /// `Bool` value.
    Bool(bool),
    /// `Int8` value.
    Int8(i8),
    /// `Int16` value.
    Int16(i16),
    /// `Int32` value.
    Int32(i32),
    /// `Int64` value.
    Int64(i64),
    /// `UInt8` value.
    UInt8(u8),
    /// `UInt16` value.
    UInt16(u16),
    /// `UInt32` value.
    UInt32(u32),
    /// `UInt64` value.
    UInt64(u64),
    /// `Float32` value.
    Float32(f32),
    /// `Float64` value.
    Float64(f64),
    /// `Text` payload.
    Text(Ptr),
    /// `Data` payload.
    Data(Ptr),
    /// List payload.
    List(Ptr),
    /// Enumerant ordinal.
    Enum(u16),
    /// Struct payload.
    Struct(Ptr),
    /// Interface value (always null).
    Interface,
    /// `AnyPointer` payload.
    AnyPointer(Ptr),
    /// Internal placeholder for a value that is evaluated in the second ("finish") phase.
    Pending(usize),
}

/// An annotation applied to a node, field, enumerant or method.
#[derive(Clone, Debug, PartialEq)]
pub struct Annotation {
    /// Id of the annotation declaration.
    pub id: Id,
    /// The annotation's value.
    pub value: Value,
    /// Generic bindings of the annotation declaration.
    pub brand: Brand,
}
