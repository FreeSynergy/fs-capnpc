//! The `Compiler` struct and its constructor (built-in nodes are registered here).
//!
//! Pattern: Facade — one object owning the node arena, loader and caches.

use crate::compiler::kinds::*;
use crate::compiler::loader_sim::LoaderSim;
use crate::compiler::node_data::*;
use crate::error::Diagnostic;
use crate::schema::{Id, Node, SourceInfo};
use crate::source::SourceLoader;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// The compiler state: node arena, id index, alias slots, loader and collected results.
pub struct Compiler {
    /// Source loader owning all parsed modules.
    pub loader: SourceLoader,
    /// Arena of declaration nodes indexed by `NodeIdx`.
    pub nodes: Vec<NodeData>,
    pub(super) nodes_by_id: HashMap<Id, usize>,
    pub(super) aliases: Vec<AliasData>,
    pub(super) builtins: HashMap<String, usize>,
    pub(super) builtins_by_kind: HashMap<DeclKind, usize>,
    pub(super) module_roots: HashMap<usize, usize>,
    pub(super) next_bogus_id: u64,
    /// Errors collected so far; compilation fails when non-empty.
    pub diagnostics: Vec<Diagnostic>,
    /// Bootstrap-state schema nodes by id.
    pub bootstrap_by_id: HashMap<Id, Rc<Node>>,
    /// Finished schema nodes by id.
    pub final_by_id: HashMap<Id, Rc<Node>>,
    /// Simulated official schema loader that fixes the output node order.
    pub sim: LoaderSim,
    /// Source info records in insertion order.
    pub source_info_inserted: Vec<SourceInfo>,
    pub(super) source_info_ids: HashSet<Id>,
    /// Files explicitly requested for compilation.
    pub requested: Vec<RequestedFile>,
}

impl Compiler {
    /// Create a compiler with the built-in declarations registered.
    pub fn new(loader: SourceLoader) -> Self {
        let mut c = Compiler {
            loader,
            nodes: vec![],
            nodes_by_id: HashMap::new(),
            aliases: vec![],
            builtins: HashMap::new(),
            builtins_by_kind: HashMap::new(),
            module_roots: HashMap::new(),
            next_bogus_id: 1000,
            diagnostics: vec![],
            bootstrap_by_id: HashMap::new(),
            final_by_id: HashMap::new(),
            sim: LoaderSim::default(),
            source_info_inserted: vec![],
            source_info_ids: HashSet::new(),
            requested: vec![],
        };
        for (i, (name, kind, gpc)) in BUILTINS.iter().enumerate() {
            let idx = c.nodes.len();
            c.nodes.push(NodeData {
                module: usize::MAX,
                parent: None,
                decl: None,
                id: 1014 + i as u64,
                display_name: name.to_string(),
                kind: *kind,
                generic_param_count: *gpc,
                is_builtin: true,
                start_byte: 0,
                end_byte: 0,
                state: State::Stub,
                in_get_content: false,
                nested: vec![],
                aliases: vec![],
                translator: None,
                bootstrap: None,
                final_set: None,
                loaded: false,
            });
            c.builtins.insert(name.to_string(), idx);
            c.builtins_by_kind.insert(*kind, idx);
        }
        c
    }
}
