//! Translator state: the work-in-progress node plus auxiliary group/param-struct nodes.
//!
//! Pattern: Builder — a `Translator` accumulates a schema node step by step until finished.

use crate::ast::*;
use crate::compiler::*;
use crate::generics::BrandScope;
use crate::schema::*;
use std::rc::Rc;

/// Definition `AuxNode`.
pub struct AuxNode {
    /// The `node` field.
    pub node: Node,
    /// The `source_info` field.
    pub source_info: SourceInfo,
}

/// Definition `Unfinished`.
pub struct Unfinished {
    /// The `source` field.
    pub source: Expression,
    /// The `ty` field.
    pub ty: Type,
    /// The `default` field.
    pub default: Value,
}

/// Definition `Translator`.
pub struct Translator {
    /// The `node_idx` field.
    pub node_idx: NodeIdx,
    /// The `module` field.
    pub module: usize,
    /// The `decl` field.
    pub decl: Rc<Declaration>,
    /// The `wip` field.
    pub wip: Node,
    /// The `source_info` field.
    pub source_info: SourceInfo,
    /// The `groups` field.
    pub groups: Vec<AuxNode>,
    /// The `param_structs` field.
    pub param_structs: Vec<AuxNode>,
    /// The `unfinished` field.
    pub unfinished: Vec<Unfinished>,
    /// The `local_brand` field.
    pub local_brand: Rc<BrandScope>,
}

impl Translator {
    /// Operation `bootstrap_set`.
    pub fn bootstrap_set(&self) -> NodeSetRc {
        let mut infos = vec![self.source_info.clone()];
        for g in &self.groups {
            infos.push(g.source_info.clone());
        }
        for p in &self.param_structs {
            infos.push(p.source_info.clone());
        }
        let is_interface = matches!(self.wip.kind, NodeKind::Interface(_));
        let aux: Vec<Rc<Node>> = if is_interface {
            self.param_structs
                .iter()
                .map(|g| Rc::new(g.node.clone()))
                .collect()
        } else {
            self.groups
                .iter()
                .map(|g| Rc::new(g.node.clone()))
                .collect()
        };
        NodeSetRc {
            node: Rc::new(self.wip.clone()),
            aux,
            source_info: infos,
        }
    }
}
