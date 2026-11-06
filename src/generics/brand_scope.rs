//! `BrandScope`: the chain of generic bindings in effect at a point of the declaration nesting.
//!
//! Pattern: Builder — scopes are built level by level with `push`, `set_params` and `pop`.

use crate::ast::*;
use crate::compiler::*;
use crate::generics::BrandedDecl;
use crate::schema::Id;
use std::rc::Rc;

/// Generic bindings in effect at each nesting level, chained to the outer scope.
pub struct BrandScope {
    pub(super) module: usize,
    pub(super) parent: Option<Rc<BrandScope>>,
    pub(super) leaf_id: Id,
    pub(super) leaf_param_count: usize,
    pub(super) inherited: bool,
    pub(super) params: Vec<BrandedDecl>,
}

impl BrandScope {
    /// Scope for `id` whose outer scopes are inherited from the resolver chain.
    pub fn new_root(
        c: &mut Compiler,
        module: usize,
        id: Id,
        param_count: usize,
        resolver: NodeIdx,
    ) -> Rc<BrandScope> {
        let parent = c
            .get_parent(resolver)
            .map(|p| BrandScope::new_root(c, module, p.id, p.generic_param_count, p.resolver));
        Rc::new(BrandScope {
            module,
            parent,
            leaf_id: id,
            leaf_param_count: param_count,
            inherited: true,
            params: vec![],
        })
    }

    pub(super) fn simple(module: usize, scope_id: Id) -> Rc<BrandScope> {
        Rc::new(BrandScope {
            module,
            parent: None,
            leaf_id: scope_id,
            leaf_param_count: 0,
            inherited: false,
            params: vec![],
        })
    }

    /// Id of the innermost scope.
    pub fn leaf_id(&self) -> Id {
        self.leaf_id
    }

    /// True when this or any outer scope has generic parameters.
    pub fn is_generic(&self) -> bool {
        self.leaf_param_count > 0 || self.parent.as_ref().is_some_and(|p| p.is_generic())
    }

    /// Enter a nested scope with `param_count` parameters.
    pub fn push(self: &Rc<Self>, type_id: Id, param_count: usize) -> Rc<BrandScope> {
        Rc::new(BrandScope {
            module: self.module,
            parent: Some(self.clone()),
            leaf_id: type_id,
            leaf_param_count: param_count,
            inherited: false,
            params: vec![],
        })
    }

    /// Bind the parameters of the innermost scope to the given declarations.
    pub fn set_params(
        self: &Rc<Self>,
        c: &mut Compiler,
        params: Vec<BrandedDecl>,
        generic_type: DeclKind,
        source: &Expression,
    ) -> Option<Rc<BrandScope>> {
        if !self.params.is_empty() {
            c.add_error(
                self.module,
                source.span,
                "Double-application of generic parameters.",
            );
            None
        } else if params.len() > self.leaf_param_count {
            if self.leaf_param_count == 0 {
                c.add_error(
                    self.module,
                    source.span,
                    "Declaration does not accept generic parameters.",
                );
            } else {
                c.add_error(self.module, source.span, "Too many generic parameters.");
            }
            None
        } else if params.len() < self.leaf_param_count {
            c.add_error(self.module, source.span, "Not enough generic parameters.");
            None
        } else {
            if generic_type != DeclKind::BuiltinList {
                for p in &params {
                    if let Some(kind) = p.kind() {
                        match kind {
                            DeclKind::BuiltinList
                            | DeclKind::BuiltinText
                            | DeclKind::BuiltinData
                            | DeclKind::BuiltinAnyPointer
                            | DeclKind::Struct
                            | DeclKind::Interface => {}
                            _ => p.add_error(
                                c,
                                self.module,
                                "Sorry, only pointer types can be used as generic parameters.",
                            ),
                        }
                    }
                }
            }
            Some(Rc::new(BrandScope {
                module: self.module,
                parent: self.parent.clone(),
                leaf_id: self.leaf_id,
                leaf_param_count: self.leaf_param_count,
                inherited: false,
                params,
            }))
        }
    }

    /// Leave scopes until the one with `new_leaf_id` is innermost.
    pub fn pop(self: &Rc<Self>, new_leaf_id: Id) -> Rc<BrandScope> {
        if self.leaf_id == new_leaf_id {
            return self.clone();
        }
        match &self.parent {
            Some(p) => p.pop(new_leaf_id),
            None => BrandScope::simple(self.module, new_leaf_id),
        }
    }
}
