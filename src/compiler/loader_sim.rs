//! Simulation of the official `SchemaLoader` insertion order.
//!
//! Pattern: Adapter — mimics the C++ loader's hash-map iteration so output order matches.

use crate::schema::{self, Brand, Id, Node};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Simulates the insertion order of the official `SchemaLoader` (a hash map that iterates in
/// insertion order, where dependencies are inserted as placeholders while a node is validated).
#[derive(Default)]
pub struct LoaderSim {
    order: Vec<Id>,
    present: HashSet<Id>,
    loaded: HashMap<Id, Rc<Node>>,
}

impl LoaderSim {
    fn touch(&mut self, id: Id) {
        if self.present.insert(id) {
            self.order.push(id);
        }
    }

    fn type_ids(ty: &schema::Type, out: &mut Vec<Id>) {
        use schema::Type::*;
        match ty {
            List(e) => Self::type_ids(e, out),
            Enum { type_id, brand } | Struct { type_id, brand } | Interface { type_id, brand } => {
                out.push(*type_id);
                Self::brand_ids(brand, out);
            }
            _ => {}
        }
    }

    fn brand_ids(brand: &Brand, out: &mut Vec<Id>) {
        for s in &brand.scopes {
            if let schema::BrandScopeKind::Bind(b) = &s.kind {
                for x in b {
                    if let schema::Binding::Type(t) = x {
                        Self::type_ids(t, out);
                    }
                }
            }
        }
    }

    /// Ids touched while validating `node`, in order.
    fn validation_ids(node: &Node) -> Vec<Id> {
        let mut out = vec![];
        match &node.kind {
            schema::NodeKind::Struct(s) => {
                for f in &s.fields {
                    match &f.kind {
                        schema::FieldKind::Slot { ty, .. } => Self::type_ids(ty, &mut out),
                        schema::FieldKind::Group { type_id } => out.push(*type_id),
                    }
                }
                if s.is_group {
                    out.push(node.scope_id);
                }
            }
            schema::NodeKind::Interface(i) => {
                for s in &i.superclasses {
                    out.push(s.id);
                    Self::brand_ids(&s.brand, &mut out);
                }
                for m in &i.methods {
                    out.push(m.param_struct_type);
                    Self::brand_ids(&m.param_brand, &mut out);
                    out.push(m.result_struct_type);
                    Self::brand_ids(&m.result_brand, &mut out);
                }
            }
            schema::NodeKind::Const { ty, .. } | schema::NodeKind::Annotation { ty, .. } => {
                Self::type_ids(ty, &mut out)
            }
            _ => {}
        }
        out
    }

    /// Load a node into the simulated loader unless it is already present.
    pub fn load_once(&mut self, node: &Rc<Node>) {
        if self.loaded.contains_key(&node.id) {
            return;
        }
        for id in Self::validation_ids(node) {
            self.touch(id);
        }
        self.touch(node.id);
        self.loaded.insert(node.id, node.clone());
    }

    /// All really loaded nodes, in insertion order.
    pub fn all_loaded(&self) -> Vec<Rc<Node>> {
        self.order
            .iter()
            .filter_map(|id| self.loaded.get(id).cloned())
            .collect()
    }
}
