//! Lazy node state transitions: expansion, bootstrap and final translation.
//!
//! Pattern: State — `ensure_state` drives a node through Stub, Expanded, Bootstrap, Finished.

use crate::ast::*;
use crate::compiler::core::Compiler;
use crate::compiler::node_data::*;
use crate::schema::{Id, Node};
use crate::translator::Translator;
use std::rc::Rc;

impl Compiler {
    /// Advance the node to at least `min`. Returns false when a recursive dependency was detected.
    pub fn ensure_state(&mut self, idx: NodeIdx, min: State) -> bool {
        assert!(
            !self.nodes[idx].is_builtin,
            "illegal method call for built-in declaration"
        );
        if self.nodes[idx].state >= min {
            return true;
        }
        if self.nodes[idx].in_get_content {
            self.node_error(idx, "Declaration recursively depends on itself.");
            return false;
        }
        self.nodes[idx].in_get_content = true;

        if self.nodes[idx].state == State::Stub && min > State::Stub {
            self.expand(idx);
        }
        if self.nodes[idx].state == State::Expanded && min > State::Expanded {
            let t = crate::translator::translate_node(self, idx);
            self.register_bootstrap(idx, &t);
            self.nodes[idx].translator = Some(t);
            self.nodes[idx].state = State::Bootstrap;
        }
        if self.nodes[idx].state == State::Bootstrap && min > State::Bootstrap {
            let t = self.nodes[idx].translator.take().expect("translator");
            let (t, set) = crate::translator::finish_translator(self, idx, t);
            self.register_final(idx, &set);
            self.nodes[idx].translator = Some(t);
            self.nodes[idx].final_set = Some(set);
            self.nodes[idx].state = State::Finished;
        }
        self.nodes[idx].in_get_content = false;
        true
    }

    pub(super) fn register_bootstrap(&mut self, idx: NodeIdx, t: &Translator) {
        let set = t.bootstrap_set();
        self.bootstrap_by_id.insert(set.node.id, set.node.clone());
        for a in &set.aux {
            self.bootstrap_by_id.insert(a.id, a.clone());
        }
        self.nodes[idx].bootstrap = Some(set);
    }

    pub(super) fn register_final(&mut self, _idx: NodeIdx, set: &NodeSetRc) {
        self.final_by_id.insert(set.node.id, set.node.clone());
        for a in &set.aux {
            self.final_by_id.insert(a.id, a.clone());
        }
    }

    pub(super) fn expand(&mut self, idx: NodeIdx) {
        let decl = self.decl(idx);
        for nested in &decl.nested {
            match &nested.body {
                DeclBody::File
                | DeclBody::Const { .. }
                | DeclBody::Annotation { .. }
                | DeclBody::Enum
                | DeclBody::Struct
                | DeclBody::Interface { .. } => {
                    let child = self.new_child(idx, nested.clone());
                    self.nodes[idx]
                        .nested
                        .push((nested.name.value.clone(), child));
                }
                DeclBody::Using { target } => {
                    let a = self.aliases.len();
                    self.aliases.push(AliasData {
                        module: self.nodes[idx].module,
                        parent: idx,
                        target_name: target.clone(),
                        target: None,
                        initialized: false,
                    });
                    self.nodes[idx].aliases.push((nested.name.value.clone(), a));
                }
                _ => {}
            }
        }
        self.nodes[idx].state = State::Expanded;
    }

    /// Bootstrap schema of a node by id (also finds group / parameter nodes of finished owners).
    pub fn resolve_bootstrap_schema(&mut self, id: Id) -> Option<Rc<Node>> {
        if let Some(idx) = self.find_node(id) {
            if let Some(f) = self.final_by_id.get(&id) {
                return Some(f.clone());
            }
            if !self.ensure_state(idx, State::Bootstrap) {
                return None;
            }
            return self.bootstrap_by_id.get(&id).cloned();
        }
        self.final_by_id
            .get(&id)
            .or_else(|| self.bootstrap_by_id.get(&id))
            .cloned()
    }

    /// Finished schema of a node by id, translating it on demand.
    pub fn resolve_final_schema(&mut self, id: Id) -> Option<Rc<Node>> {
        if let Some(idx) = self.find_node(id) {
            if !self.ensure_state(idx, State::Finished) {
                return None;
            }
            return self.final_by_id.get(&id).cloned();
        }
        self.final_by_id.get(&id).cloned()
    }
}
