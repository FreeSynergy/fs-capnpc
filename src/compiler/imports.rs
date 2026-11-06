//! Import discovery: collects the `import` names used by a file for the request.
//!
//! Pattern: Visitor — recursive walk over declarations and expressions.

use crate::ast::*;
use crate::compiler::core::Compiler;
use crate::schema::Id;
use std::collections::BTreeSet;

impl Compiler {
    /// Collect the (id, name) import table of a module, loading each import.
    pub fn file_import_table(&mut self, module: usize) -> Vec<(Id, String)> {
        let mut names = BTreeSet::new();
        find_imports_decl(&self.loader.modules[module].root.clone(), &mut names);
        let mut out = vec![];
        for n in names {
            let mut errs = vec![];
            if let Some(m) = self.loader.import_relative(module, &n, &mut errs) {
                let root = self.add_module(m);
                out.push((self.nodes[root].id, n));
            }
        }
        out
    }
}

fn find_imports_expr(e: &Expression, out: &mut BTreeSet<String>) {
    match &e.kind {
        ExprKind::Import(n) => {
            out.insert(n.clone());
        }
        ExprKind::List(l) => {
            for x in l {
                find_imports_expr(x, out);
            }
        }
        ExprKind::Tuple(t) => {
            for p in t {
                find_imports_expr(&p.value, out);
            }
        }
        ExprKind::Application { function, params } => {
            find_imports_expr(function, out);
            for p in params {
                find_imports_expr(&p.value, out);
            }
        }
        ExprKind::Member { parent, .. } => find_imports_expr(parent, out),
        _ => {}
    }
}

fn find_imports_params(pl: &ParamList, out: &mut BTreeSet<String>) {
    match &pl.kind {
        ParamListKind::Named(ps) => {
            for p in ps {
                find_imports_expr(&p.ty, out);
                for a in &p.annotations {
                    find_imports_expr(&a.name, out);
                }
            }
        }
        ParamListKind::Type(t) => find_imports_expr(t, out),
        ParamListKind::Stream => {
            out.insert("/capnp/stream.capnp".to_string());
        }
    }
}

fn find_imports_decl(d: &Declaration, out: &mut BTreeSet<String>) {
    match &d.body {
        DeclBody::Using { target } => find_imports_expr(target, out),
        DeclBody::Const { ty, .. } => find_imports_expr(ty, out),
        DeclBody::Field { ty, .. } => find_imports_expr(ty, out),
        DeclBody::Interface { superclasses } => {
            for s in superclasses {
                find_imports_expr(s, out);
            }
        }
        DeclBody::Method { params, results } => {
            find_imports_params(params, out);
            if let Some(r) = results {
                find_imports_params(r, out);
            }
        }
        _ => {}
    }
    for a in &d.annotations {
        find_imports_expr(&a.name, out);
    }
    for n in &d.nested {
        find_imports_decl(n, out);
    }
}
