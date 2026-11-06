//! Per-node compiler state: lazy translation states, node data and alias slots.
//!
//! Pattern: State — a node advances Stub, Expanded, Bootstrap, Finished on demand.

use crate::ast::*;
use crate::compiler::kinds::DeclKind;
use crate::compiler::resolved::ResolveResult;
use crate::schema::{Id, Node, SourceInfo};
use crate::translator::Translator;
use std::rc::Rc;

/// Translation state of a node; each state implies the previous ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum State {
    /// Created, nested names not yet known.
    Stub,
    /// Nested declarations and aliases registered.
    Expanded,
    /// Bootstrap node translated.
    Bootstrap,
    /// Final node translated.
    Finished,
}

/// A translated declaration: node plus auxiliary nodes (groups / method parameter structs).
#[derive(Clone, Debug)]
pub struct NodeSetRc {
    /// The main node.
    pub node: Rc<Node>,
    /// Auxiliary group and parameter-struct nodes.
    pub aux: Vec<Rc<Node>>,
    /// Source info for the nodes.
    pub source_info: Vec<SourceInfo>,
}

/// One declaration node in the compiler arena.
pub struct NodeData {
    /// Index of the module the node belongs to.
    pub module: usize,
    /// Parent node, `None` for roots.
    pub parent: Option<usize>,
    /// Parsed declaration, `None` for built-ins.
    pub decl: Option<Rc<Declaration>>,
    /// Schema id.
    pub id: Id,
    /// Fully qualified display name.
    pub display_name: String,
    /// Declaration kind.
    pub kind: DeclKind,
    /// Number of generic parameters.
    pub generic_param_count: usize,
    /// True for built-in types.
    pub is_builtin: bool,
    /// Start of the name span in the source.
    pub start_byte: u32,
    /// End of the name span in the source.
    pub end_byte: u32,
    /// Current translation state.
    pub state: State,
    /// Re-entrancy guard for recursion detection.
    pub in_get_content: bool,
    /// Nested declarations by name.
    pub nested: Vec<(String, usize)>,
    /// `using` aliases by name.
    pub aliases: Vec<(String, usize)>,
    /// Translator kept after the bootstrap state.
    pub translator: Option<Translator>,
    /// Bootstrap node set.
    pub bootstrap: Option<NodeSetRc>,
    /// Final node set.
    pub final_set: Option<NodeSetRc>,
    /// The final node set has been loaded into the (simulated) final schema loader.
    pub loaded: bool,
}

pub(super) struct AliasData {
    pub(super) module: usize,
    pub(super) parent: usize,
    pub(super) target_name: Expression,
    pub(super) target: Option<ResolveResult>,
    pub(super) initialized: bool,
}

/// A file requested on the command line.
pub struct RequestedFile {
    /// Schema id.
    pub id: Id,
    /// Display file name.
    pub filename: String,
    /// Imports as (id, name).
    pub imports: Vec<(Id, String)>,
    /// Index of the module the node belongs to.
    pub module: usize,
}

/// Index of a node in `Compiler::nodes`.
pub type NodeIdx = usize;
