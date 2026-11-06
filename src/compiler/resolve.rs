//! Name resolution: scoped lookup, `using` aliases, imports and declaration compilation.
//!
//! Pattern: Registry — looks names up through nested scopes, aliases and the built-in table.

use crate::ast::*;
use crate::compiler::core::Compiler;
use crate::compiler::kinds::DeclKind;
use crate::compiler::node_data::*;
use crate::compiler::resolved::*;
use crate::generics::BrandScope;
use crate::schema::Id;

impl Compiler {
    pub(super) fn resolved(&self, idx: NodeIdx, scope_id: Id) -> ResolvedDecl {
        let n = &self.nodes[idx];
        ResolvedDecl {
            id: n.id,
            generic_param_count: n.generic_param_count,
            scope_id,
            kind: n.kind,
            resolver: idx,
            brand: None,
        }
    }

    /// Resolve `name` starting at `node`, walking outward through parents and built-ins.
    pub fn resolve(&mut self, node: NodeIdx, name: &str) -> Option<ResolveResult> {
        if let Some(r) = self.resolve_member(node, name) {
            return Some(r);
        }
        if let Some(d) = &self.nodes[node].decl {
            for (i, p) in d.parameters.iter().enumerate() {
                if p.name == name {
                    return Some(ResolveResult::Param(ResolvedParameter {
                        id: self.nodes[node].id,
                        index: i,
                    }));
                }
            }
        }
        if let Some(p) = self.nodes[node].parent {
            self.resolve(p, name)
        } else if let Some(&b) = self.builtins.get(name) {
            Some(ResolveResult::Decl(self.resolved(b, 0)))
        } else {
            None
        }
    }

    /// Resolve `name` as a direct member (nested declaration or alias) of `node`.
    pub fn resolve_member(&mut self, node: NodeIdx, name: &str) -> Option<ResolveResult> {
        if self.nodes[node].is_builtin {
            return None;
        }
        if !self.ensure_state(node, State::Expanded) {
            return None;
        }
        if let Some((_, c)) = self.nodes[node].nested.iter().find(|(n, _)| n == name) {
            let c = *c;
            let sid = self.nodes[node].id;
            return Some(ResolveResult::Decl(self.resolved(c, sid)));
        }
        if let Some((_, a)) = self.nodes[node].aliases.iter().find(|(n, _)| n == name) {
            let a = *a;
            return self.alias_compile(a);
        }
        None
    }

    /// Resolve a built-in type by kind.
    pub fn resolve_builtin(&self, kind: DeclKind) -> ResolvedDecl {
        self.resolved(self.builtins_by_kind[&kind], 0)
    }

    /// Resolve a declaration by schema id.
    pub fn resolve_id(&self, id: Id) -> ResolvedDecl {
        let n = self.find_node(id).expect("resolve_id: unknown id");
        let pid = self.nodes[n].parent.map(|p| self.nodes[p].id).unwrap_or(0);
        self.resolved(n, pid)
    }

    /// The resolved parent declaration of `node`, if any.
    pub fn get_parent(&self, node: NodeIdx) -> Option<ResolvedDecl> {
        self.nodes[node].parent.map(|p| {
            let scope = self.nodes[p].parent.map(|g| self.nodes[g].id).unwrap_or(0);
            self.resolved(p, scope)
        })
    }

    /// The resolved file root containing `node`.
    pub fn get_top_scope(&self, node: NodeIdx) -> ResolvedDecl {
        let root = self.module_roots[&self.nodes[node].module];
        self.resolved(root, 0)
    }

    /// Resolve an `import` expression relative to `node`'s module.
    pub fn resolve_import(&mut self, node: NodeIdx, name: &str) -> Option<ResolvedDecl> {
        let module = self.nodes[node].module;
        let mut errs = vec![];
        let m = self.loader.import_relative(module, name, &mut errs);
        self.diagnostics.extend(errs);
        let m = m?;
        let root = self.add_module(m);
        Some(self.resolved(root, 0))
    }

    /// Read an `embed` file relative to `node`'s module.
    pub fn read_embed(&self, node: NodeIdx, name: &str) -> Option<Vec<u8>> {
        self.loader.embed_relative(self.nodes[node].module, name)
    }

    /// Resolve (once) the target of a `using` alias.
    pub fn alias_compile(&mut self, a: usize) -> Option<ResolveResult> {
        if !self.aliases[a].initialized {
            self.aliases[a].initialized = true;
            let (module, parent, expr) = (
                self.aliases[a].module,
                self.aliases[a].parent,
                self.aliases[a].target_name.clone(),
            );
            let pid = self.nodes[parent].id;
            let pcount = self.nodes[parent].generic_param_count;
            let r = self.compile_decl(pid, pcount, parent, module, &expr);
            self.aliases[a].target = r;
        }
        self.aliases[a].target.clone()
    }

    /// `NodeTranslator::compileDecl`.
    pub fn compile_decl(
        &mut self,
        scope_id: Id,
        scope_param_count: usize,
        resolver: NodeIdx,
        module: usize,
        expr: &Expression,
    ) -> Option<ResolveResult> {
        let scope = BrandScope::new_root(self, module, scope_id, scope_param_count, resolver);
        let decl = scope.compile_decl_expression(
            self,
            expr,
            resolver,
            &crate::generics::ImplicitParams::none(),
        )?;
        Some(decl.as_resolve_result(self, scope.leaf_id()))
    }
}
