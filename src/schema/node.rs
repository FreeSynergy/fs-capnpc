//! Schema nodes and the sets the translator produces.
//!
//! Pattern: Composite — a node's kind holds kind-specific data; sets bundle a node with its helpers.

use super::brand::Brand;
use super::field::{Enumerant, Field};
use super::source_info::SourceInfo;
use super::types::{Id, Type};
use super::values::{Annotation, Value};
use crate::ast::Targets;

/// Struct-specific part of a node.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StructNode {
    /// Size of the data section in words.
    pub data_word_count: u16,
    /// Number of pointer slots.
    pub pointer_count: u16,
    /// Whether the struct is a group.
    pub is_group: bool,
    /// Number of union members.
    pub discriminant_count: u16,
    /// Offset of the union tag, in 16-bit units.
    pub discriminant_offset: u32,
    /// Fields in declaration order.
    pub fields: Vec<Field>,
}

/// A base interface of an interface node.
#[derive(Clone, Debug, PartialEq)]
pub struct Superclass {
    /// Id of the base interface.
    pub id: Id,
    /// Generic bindings of the base interface.
    pub brand: Brand,
}

/// One method of an interface.
#[derive(Clone, Debug, PartialEq)]
pub struct Method {
    /// Method name.
    pub name: String,
    /// Position of the method in the source.
    pub code_order: u16,
    /// Names of implicit generic parameters.
    pub implicit_parameters: Vec<String>,
    /// Id of the parameter struct.
    pub param_struct_type: Id,
    /// Generic bindings of the parameter struct.
    pub param_brand: Brand,
    /// Id of the result struct.
    pub result_struct_type: Id,
    /// Generic bindings of the result struct.
    pub result_brand: Brand,
    /// Annotations applied to the method.
    pub annotations: Vec<Annotation>,
}

/// Interface-specific part of a node.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InterfaceNode {
    /// Methods in declaration order.
    pub methods: Vec<Method>,
    /// Base interfaces.
    pub superclasses: Vec<Superclass>,
}

/// The kind-specific part of a [`Node`].
#[derive(Clone, Debug, PartialEq)]
pub enum NodeKind {
    /// A file node.
    File,
    /// A struct or group.
    Struct(StructNode),
    /// An enum with its enumerants.
    Enum(Vec<Enumerant>),
    /// An interface.
    Interface(InterfaceNode),
    /// A constant.
    Const {
        /// Constant type.
        ty: Type,
        /// Constant value.
        value: Value,
    },
    /// An annotation declaration.
    Annotation {
        /// Type of the annotation value.
        ty: Type,
        /// Declaration kinds it may be applied to.
        targets: Targets,
    },
}

/// A schema node.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// Unique id.
    pub id: Id,
    /// Fully qualified display name.
    pub display_name: String,
    /// Length of the scope prefix within `display_name`.
    pub display_name_prefix_length: u32,
    /// Id of the enclosing node.
    pub scope_id: Id,
    /// `None` for synthetic nodes (groups, parameter structs) where the official compiler leaves the
    /// list pointer null.
    pub nested_nodes: Option<Vec<(String, Id)>>,
    /// Annotations applied to the node.
    pub annotations: Vec<Annotation>,
    /// Names of generic parameters.
    pub parameters: Vec<String>,
    /// Whether the node has generic parameters in scope.
    pub is_generic: bool,
    /// Byte offset where the node starts.
    pub start_byte: u32,
    /// Byte offset where the node ends.
    pub end_byte: u32,
    /// Kind-specific data.
    pub kind: NodeKind,
}

impl Node {
    /// Whether this node is a struct (or group).
    pub fn is_struct(&self) -> bool {
        matches!(self.kind, NodeKind::Struct(_))
    }

    /// The struct data if this node is a struct.
    pub fn struct_node(&self) -> Option<&StructNode> {
        match &self.kind {
            NodeKind::Struct(s) => Some(s),
            _ => None,
        }
    }
}

/// The result of translating one declaration: the node, its auxiliary nodes (groups or method
/// parameter structs) and source info records.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeSet {
    /// The declaration's own node.
    pub node: Node,
    /// Group and parameter-struct nodes created alongside it.
    pub aux_nodes: Vec<Node>,
    /// Source info records for the node and its auxiliaries.
    pub source_info: Vec<SourceInfo>,
}
