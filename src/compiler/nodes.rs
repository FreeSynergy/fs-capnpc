//! Node arena management: module roots, child creation and id registration.
//!
//! Pattern: Builder — assembles `NodeData` entries for modules and nested declarations.

use crate::ast::*;
use crate::compiler::core::Compiler;
use crate::compiler::kinds::{DeclKind, decl_kind};
use crate::compiler::node_data::*;
use crate::error::Span;
use crate::ids::generate_child_id;
use crate::schema::Id;
use std::rc::Rc;

impl Compiler {
    /// Make sure `module` has a root node and return it.
    pub fn add_module(&mut self, module: usize) -> NodeIdx {
        if let Some(&r) = self.module_roots.get(&module) {
            return r;
        }
        let decl = self.loader.modules[module].root.clone();
        let display = self.loader.modules[module].display_name.clone();
        let id = decl.uid().unwrap_or(0);
        let idx = self.nodes.len();
        self.nodes.push(NodeData {
            module,
            parent: None,
            kind: DeclKind::File,
            generic_param_count: decl.parameters.len(),
            is_builtin: false,
            start_byte: decl.span.start,
            end_byte: decl.span.end,
            state: State::Stub,
            in_get_content: false,
            nested: vec![],
            aliases: vec![],
            translator: None,
            bootstrap: None,
            final_set: None,
            loaded: false,
            id,
            display_name: display,
            decl: Some(decl),
        });
        self.module_roots.insert(module, idx);
        let final_id = self.add_node(id, idx);
        self.nodes[idx].id = final_id;
        idx
    }

    pub(super) fn new_child(&mut self, parent: NodeIdx, decl: Rc<Declaration>) -> NodeIdx {
        let module = self.nodes[parent].module;
        let pid = self.nodes[parent].id;
        let id = decl
            .uid()
            .unwrap_or_else(|| generate_child_id(pid, &decl.name.value));
        let sep = if self.nodes[parent].parent.is_none() {
            ':'
        } else {
            '.'
        };
        let display = format!(
            "{}{}{}",
            self.nodes[parent].display_name, sep, decl.name.value
        );
        let span = if decl.name.value.is_empty() {
            decl.span
        } else {
            decl.name.span
        };
        let idx = self.nodes.len();
        self.nodes.push(NodeData {
            module,
            parent: Some(parent),
            kind: decl_kind(&decl),
            generic_param_count: decl.parameters.len(),
            is_builtin: false,
            start_byte: span.start,
            end_byte: span.end,
            state: State::Stub,
            in_get_content: false,
            nested: vec![],
            aliases: vec![],
            translator: None,
            bootstrap: None,
            final_set: None,
            loaded: false,
            id,
            display_name: display,
            decl: Some(decl),
        });
        let final_id = self.add_node(id, idx);
        self.nodes[idx].id = final_id;
        idx
    }

    pub(super) fn add_node(&mut self, desired: Id, idx: NodeIdx) -> Id {
        let mut desired = desired;
        loop {
            if let std::collections::hash_map::Entry::Vacant(e) = self.nodes_by_id.entry(desired) {
                e.insert(idx);
                return desired;
            }
            if desired & (1u64 << 63) != 0 {
                let other = self.nodes_by_id[&desired];
                let (m, s, e) = (
                    self.nodes[idx].module,
                    self.nodes[idx].start_byte,
                    self.nodes[idx].end_byte,
                );
                self.add_error(m, Span::new(s, e), format!("Duplicate ID @0x{desired:x}."));
                let (m2, s2, e2) = (
                    self.nodes[other].module,
                    self.nodes[other].start_byte,
                    self.nodes[other].end_byte,
                );
                self.add_error(
                    m2,
                    Span::new(s2, e2),
                    format!("ID @0x{desired:x} originally used here."),
                );
            }
            desired = self.next_bogus_id;
            self.next_bogus_id += 1;
        }
    }

    /// Look up a node index by schema id.
    pub fn find_node(&self, id: Id) -> Option<NodeIdx> {
        self.nodes_by_id.get(&id).copied()
    }

    /// The parsed declaration of a node (panics for built-ins).
    pub fn decl(&self, idx: NodeIdx) -> Rc<Declaration> {
        self.nodes[idx]
            .decl
            .clone()
            .expect("builtin has no declaration")
    }

    /// Report an error at the node's name span.
    pub fn node_error(&mut self, idx: NodeIdx, msg: impl Into<String>) {
        let (m, s, e) = (
            self.nodes[idx].module,
            self.nodes[idx].start_byte,
            self.nodes[idx].end_byte,
        );
        self.add_error(m, Span::new(s, e), msg);
    }
}
