//! Compilation of declaration expressions (names, members, applications, imports).
//!
//! Pattern: Visitor — recursive dispatch over the expression kinds.

use crate::ast::*;
use crate::compiler::*;
use crate::generics::{BrandScope, BrandedDecl, ImplicitParams};
use crate::parser::expression_string;
use std::rc::Rc;

impl BrandScope {
    /// Compile a declaration expression (name, member, import, application) to a branded declaration.
    pub fn compile_decl_expression(
        self: &Rc<Self>,
        c: &mut Compiler,
        source: &Expression,
        resolver: NodeIdx,
        implicit: &ImplicitParams,
    ) -> Option<BrandedDecl> {
        match &source.kind {
            ExprKind::Unknown => None,
            ExprKind::PositiveInt(_)
            | ExprKind::NegativeInt(_)
            | ExprKind::Float(_)
            | ExprKind::String(_)
            | ExprKind::Binary(_)
            | ExprKind::List(_)
            | ExprKind::Tuple(_)
            | ExprKind::Embed(_) => {
                c.add_error(self.module, source.span, "Expected name.");
                None
            }
            ExprKind::RelativeName(name) => {
                for (i, p) in implicit.params.iter().enumerate() {
                    if p == name {
                        return Some(if implicit.scope_id == 0 {
                            BrandedDecl::implicit_method_param(i)
                        } else {
                            BrandedDecl {
                                body: ResolveResult::Param(ResolvedParameter {
                                    id: implicit.scope_id,
                                    index: i,
                                }),
                                brand: None,
                                source: None,
                            }
                        });
                    }
                }
                if let Some(r) = c.resolve(resolver, name) {
                    let result = self.interpret_resolve(c, &r, source);
                    if let ResolveResult::Decl(d) = &r {
                        c.report_resolution(self.module, source.span.start, source.span.end, d.id);
                    }
                    Some(result)
                } else {
                    c.add_error(self.module, source.span, format!("Not defined: {name}"));
                    None
                }
            }
            ExprKind::AbsoluteName(name) => {
                let top = c.get_top_scope(resolver);
                if let Some(r) = c.resolve_member(top.resolver, name) {
                    let result = self.interpret_resolve(c, &r, source);
                    if let ResolveResult::Decl(d) = &r {
                        c.report_resolution(self.module, source.span.start, source.span.end, d.id);
                    }
                    Some(result)
                } else {
                    c.add_error(self.module, source.span, format!("Not defined: {name}"));
                    None
                }
            }
            ExprKind::Import(filename) => {
                if let Some(decl) = c.resolve_import(resolver, filename) {
                    c.report_resolution(self.module, source.span.start, source.span.end, decl.id);
                    let scope = BrandScope::new_root(
                        c,
                        self.module,
                        decl.id,
                        decl.generic_param_count,
                        decl.resolver,
                    );
                    Some(BrandedDecl {
                        body: ResolveResult::Decl(decl),
                        brand: Some(scope),
                        source: Some(Rc::new(source.clone())),
                    })
                } else {
                    c.add_error(
                        self.module,
                        source.span,
                        format!("Import failed: {filename}"),
                    );
                    None
                }
            }
            ExprKind::Application { function, params } => {
                let decl = self.compile_decl_expression(c, function, resolver, implicit)?;
                let mut compiled = vec![];
                let mut failed = false;
                for p in params {
                    if let Some(n) = &p.name {
                        c.add_error(self.module, n.span, "Named parameter not allowed here.");
                    }
                    match self.compile_decl_expression(c, &p.value, resolver, implicit) {
                        Some(d) => compiled.push(d),
                        None => failed = true,
                    }
                }
                if failed {
                    return Some(decl);
                }
                match decl.apply_params(c, compiled, source) {
                    Some(a) => Some(a),
                    None => Some(decl),
                }
            }
            ExprKind::Member { parent, name } => {
                let decl = self.compile_decl_expression(c, parent, resolver, implicit)?;
                if let Some(m) = decl.get_member(c, &name.value, source) {
                    if let Some(id) = m.generic_type_id() {
                        c.report_resolution(self.module, source.span.start, source.span.end, id);
                    }
                    Some(m)
                } else {
                    c.add_error(
                        self.module,
                        name.span,
                        format!(
                            "'{}' has no member named '{}'",
                            expression_string(parent),
                            name.value
                        ),
                    );
                    None
                }
            }
        }
    }
}
