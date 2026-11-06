//! Dependency traversal that compiles nodes eagerly and collects source info.
//!
//! Pattern: Visitor — walks nodes, types, brands and annotations to reach every dependency.

use crate::compiler::core::Compiler;
use crate::compiler::kinds::*;
use crate::compiler::node_data::*;
use crate::schema::{self, Brand, Id, Node, SourceInfo};
use std::collections::HashMap;

impl Compiler {
    /// Translate `root` and everything `eagerness` asks for, collecting source info.
    pub fn eagerly_compile(&mut self, root: NodeIdx, eagerness: u32) {
        let mut seen: HashMap<NodeIdx, u32> = HashMap::new();
        let mut infos: Vec<SourceInfo> = vec![];
        self.traverse(root, eagerness, &mut seen, &mut infos);
        for si in infos {
            if self.source_info_ids.insert(si.id) {
                self.source_info_inserted.push(si);
            }
        }
    }

    pub(super) fn load_final_schema(&mut self, idx: NodeIdx) {
        if self.nodes[idx].loaded {
            return;
        }
        if let Some(set) = self.nodes[idx].final_set.clone() {
            for a in &set.aux {
                self.sim.load_once(a);
            }
            self.sim.load_once(&set.node);
            self.nodes[idx].loaded = true;
        }
    }

    pub(super) fn traverse(
        &mut self,
        idx: NodeIdx,
        eagerness: u32,
        seen: &mut HashMap<NodeIdx, u32>,
        si: &mut Vec<SourceInfo>,
    ) {
        let slot = seen.entry(idx).or_insert(0);
        if *slot & eagerness == eagerness {
            return;
        }
        *slot |= eagerness;

        if self.ensure_state(idx, State::Finished) {
            self.load_final_schema(idx);
            if let Some(set) = self.nodes[idx].final_set.clone() {
                if eagerness / DEPENDENCIES != 0 {
                    let new_e = (eagerness & !(DEPENDENCIES - 1)) | (eagerness / DEPENDENCIES);
                    self.traverse_node_dependencies(&set.node, new_e, seen, si);
                    for a in &set.aux {
                        self.traverse_node_dependencies(a, new_e, seen, si);
                    }
                }
                si.extend(set.source_info.iter().cloned());
            }
        }

        if eagerness & PARENTS != 0
            && let Some(p) = self.nodes[idx].parent
        {
            self.traverse(p, eagerness, seen, si);
        }

        if eagerness & CHILDREN != 0 && self.ensure_state(idx, State::Expanded) {
            let children: Vec<usize> = self.nodes[idx].nested.iter().map(|(_, c)| *c).collect();
            for c in children {
                self.traverse(c, eagerness, seen, si);
            }
            let aliases: Vec<usize> = self.nodes[idx].aliases.iter().map(|(_, a)| *a).collect();
            for a in aliases {
                self.alias_compile(a);
            }
        }
    }

    pub(super) fn traverse_node_dependencies(
        &mut self,
        node: &Node,
        e: u32,
        seen: &mut HashMap<NodeIdx, u32>,
        si: &mut Vec<SourceInfo>,
    ) {
        use schema::NodeKind::*;
        match &node.kind {
            Struct(s) => {
                for f in &s.fields {
                    if let schema::FieldKind::Slot { ty, .. } = &f.kind {
                        self.traverse_type(ty, e, seen, si);
                    }
                    self.traverse_annotations(&f.annotations, e, seen, si);
                }
            }
            Enum(en) => {
                for x in en {
                    self.traverse_annotations(&x.annotations, e, seen, si);
                }
            }
            Interface(i) => {
                for s in &i.superclasses {
                    if s.id != 0 {
                        self.traverse_dependency(s.id, e, seen, si, false);
                    }
                    self.traverse_brand(&s.brand, e, seen, si);
                }
                for m in &i.methods {
                    self.traverse_dependency(m.param_struct_type, e, seen, si, true);
                    self.traverse_brand(&m.param_brand, e, seen, si);
                    self.traverse_dependency(m.result_struct_type, e, seen, si, true);
                    self.traverse_brand(&m.result_brand, e, seen, si);
                    self.traverse_annotations(&m.annotations, e, seen, si);
                }
            }
            Const { ty, .. } => self.traverse_type(ty, e, seen, si),
            Annotation { ty, .. } => self.traverse_type(ty, e, seen, si),
            File => {}
        }
        self.traverse_annotations(&node.annotations, e, seen, si);
    }

    pub(super) fn traverse_type(
        &mut self,
        ty: &schema::Type,
        e: u32,
        seen: &mut HashMap<NodeIdx, u32>,
        si: &mut Vec<SourceInfo>,
    ) {
        use schema::Type::*;
        match ty {
            Struct { type_id, brand } | Enum { type_id, brand } | Interface { type_id, brand } => {
                self.traverse_dependency(*type_id, e, seen, si, false);
                self.traverse_brand(brand, e, seen, si);
            }
            List(el) => self.traverse_type(el, e, seen, si),
            _ => {}
        }
    }

    pub(super) fn traverse_brand(
        &mut self,
        brand: &Brand,
        e: u32,
        seen: &mut HashMap<NodeIdx, u32>,
        si: &mut Vec<SourceInfo>,
    ) {
        for s in &brand.scopes {
            if let schema::BrandScopeKind::Bind(bs) = &s.kind {
                for b in bs {
                    if let schema::Binding::Type(t) = b {
                        self.traverse_type(t, e, seen, si);
                    }
                }
            }
        }
    }

    pub(super) fn traverse_annotations(
        &mut self,
        anns: &[schema::Annotation],
        e: u32,
        seen: &mut HashMap<NodeIdx, u32>,
        si: &mut Vec<SourceInfo>,
    ) {
        for a in anns {
            if let Some(n) = self.find_node(a.id) {
                self.traverse(n, e, seen, si);
            }
        }
    }

    pub(super) fn traverse_dependency(
        &mut self,
        id: Id,
        e: u32,
        seen: &mut HashMap<NodeIdx, u32>,
        si: &mut Vec<SourceInfo>,
        ignore_if_not_found: bool,
    ) {
        if let Some(n) = self.find_node(id) {
            self.traverse(n, e, seen, si);
        } else if !ignore_if_not_found {
            // Dependency ID not present in compiler; nothing to traverse.
        }
    }
}
