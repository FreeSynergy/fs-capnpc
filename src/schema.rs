//! In-memory form of the schema nodes the compiler produces (mirrors `schema.capnp`).
//!
//! The translator builds these plain structs; [`crate::request`] serialises them with the
//! `schema_capnp` builders. Where the official compiler distinguishes an absent list pointer from an
//! empty one (observable in the encoded message), the corresponding field here is an `Option`.

pub type Id = u64;

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Void,
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float32,
    Float64,
    Text,
    Data,
    List(Box<Type>),
    Enum { type_id: Id, brand: Brand },
    Struct { type_id: Id, brand: Brand },
    Interface { type_id: Id, brand: Brand },
    AnyPointer(AnyPointer),
}

#[derive(Clone, Debug, PartialEq)]
pub enum AnyPointer {
    Unconstrained(Unconstrained),
    Parameter { scope_id: Id, parameter_index: u16 },
    ImplicitMethodParameter { parameter_index: u16 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unconstrained {
    AnyKind,
    Struct,
    List,
    Capability,
}

/// A brand; an empty `scopes` list means "no brand" (the pointer is left null).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Brand {
    pub scopes: Vec<BrandScope>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrandScope {
    pub scope_id: Id,
    pub kind: BrandScopeKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BrandScopeKind {
    Bind(Vec<Binding>),
    Inherit,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Binding {
    Unbound,
    Type(Type),
}

/// A pointer-typed value tree (the payload of `Value.text`, `.data`, `.list`, `.struct`, ...).
#[derive(Clone, Debug, PartialEq)]
pub enum Ptr {
    Null,
    Text(Vec<u8>),
    Data(Vec<u8>),
    Struct(StructVal),
    List(ListVal),
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructVal {
    pub data_words: u16,
    pub pointer_count: u16,
    /// `data_words * 8` bytes.
    pub data: Vec<u8>,
    pub ptrs: Vec<Ptr>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ListVal {
    Void(u32),
    Bits(Vec<bool>),
    /// Element size in bytes (1, 2, 4 or 8) and little-endian values.
    Bytes { size: u8, values: Vec<u64> },
    Pointers(Vec<Ptr>),
    Structs { data_words: u16, pointer_count: u16, elems: Vec<StructVal> },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Void,
    Bool(bool),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    Float32(f32),
    Float64(f64),
    Text(Ptr),
    Data(Ptr),
    List(Ptr),
    Enum(u16),
    Struct(Ptr),
    Interface,
    AnyPointer(Ptr),
    /// Internal placeholder for a value that is evaluated in the second ("finish") phase.
    Pending(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Annotation {
    pub id: Id,
    pub value: Value,
    pub brand: Brand,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ordinal {
    Implicit,
    Explicit(u16),
}

#[derive(Clone, Debug, PartialEq)]
pub enum FieldKind {
    Slot { offset: u32, ty: Type, default_value: Value, had_explicit_default: bool },
    Group { type_id: Id },
}

pub const NO_DISCRIMINANT: u16 = 0xffff;

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub name: String,
    pub code_order: u16,
    pub annotations: Vec<Annotation>,
    pub discriminant_value: u16,
    pub kind: FieldKind,
    pub ordinal: Ordinal,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StructNode {
    pub data_word_count: u16,
    pub pointer_count: u16,
    pub is_group: bool,
    pub discriminant_count: u16,
    pub discriminant_offset: u32,
    pub fields: Vec<Field>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Enumerant {
    pub name: String,
    pub code_order: u16,
    pub annotations: Vec<Annotation>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Superclass {
    pub id: Id,
    pub brand: Brand,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Method {
    pub name: String,
    pub code_order: u16,
    pub implicit_parameters: Vec<String>,
    pub param_struct_type: Id,
    pub param_brand: Brand,
    pub result_struct_type: Id,
    pub result_brand: Brand,
    pub annotations: Vec<Annotation>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct InterfaceNode {
    pub methods: Vec<Method>,
    pub superclasses: Vec<Superclass>,
}

pub use crate::ast::Targets;

#[derive(Clone, Debug, PartialEq)]
pub enum NodeKind {
    File,
    Struct(StructNode),
    Enum(Vec<Enumerant>),
    Interface(InterfaceNode),
    Const { ty: Type, value: Value },
    Annotation { ty: Type, targets: Targets },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: Id,
    pub display_name: String,
    pub display_name_prefix_length: u32,
    pub scope_id: Id,
    /// `None` for synthetic nodes (groups, parameter structs) where the official compiler leaves the
    /// list pointer null.
    pub nested_nodes: Option<Vec<(String, Id)>>,
    pub annotations: Vec<Annotation>,
    pub parameters: Vec<String>,
    pub is_generic: bool,
    pub start_byte: u32,
    pub end_byte: u32,
    pub kind: NodeKind,
}

impl Node {
    pub fn is_struct(&self) -> bool {
        matches!(self.kind, NodeKind::Struct(_))
    }
    pub fn struct_node(&self) -> Option<&StructNode> {
        match &self.kind {
            NodeKind::Struct(s) => Some(s),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MemberSourceInfo {
    pub doc_comment: Option<String>,
    pub start_byte: u32,
    pub end_byte: u32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SourceInfo {
    pub id: Id,
    pub doc_comment: Option<String>,
    /// `None` when the official compiler never initialises the list.
    pub members: Option<Vec<MemberSourceInfo>>,
    pub start_byte: u32,
    pub end_byte: u32,
}

/// The result of translating one declaration: the node, its auxiliary nodes (groups or method
/// parameter structs) and source info records.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeSet {
    pub node: Node,
    pub aux_nodes: Vec<Node>,
    pub source_info: Vec<SourceInfo>,
}
