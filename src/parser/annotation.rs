//! Annotation applications and annotation declarations.
//!
//! Pattern: Template method — shared token grammar for `$annotation(value)` and `annotation` decls.

use super::cursor::P;
use crate::ast::*;

impl P<'_> {
    pub(super) fn annotation(&mut self) -> Option<AnnotationApplication> {
        self.attempt(|p| {
            p.op("$")?;
            let e = p.expression()?;
            Some(match e.kind {
                ExprKind::Application {
                    function,
                    mut params,
                } => {
                    if params.len() == 1 && params[0].name.is_none() {
                        AnnotationApplication {
                            name: *function,
                            value: Some(params.pop().unwrap().value),
                        }
                    } else {
                        let span = e.span;
                        AnnotationApplication {
                            name: *function,
                            value: Some(Expression {
                                kind: ExprKind::Tuple(params),
                                span,
                            }),
                        }
                    }
                }
                _ => AnnotationApplication {
                    name: e,
                    value: None,
                },
            })
        })
    }

    pub(super) fn annotations(&mut self) -> Vec<AnnotationApplication> {
        let mut v = vec![];
        while let Some(a) = self.annotation() {
            v.push(a);
        }
        v
    }

    pub(super) fn annotation_decl(&mut self) -> Option<Declaration> {
        self.attempt(|p| {
            p.keyword("annotation")?;
            let name = p.ident()?;
            let id = p.uid();
            let (items, span) = p.paren_items()?;
            let targets_raw = p.list_items(items, span, |p| {
                if let Some(s) = p.op("*") {
                    return Some(Located::new("*".to_string(), s));
                }
                p.ident()
            });
            p.op(":")?;
            let ty = p.expression()?;
            let annotations = p.annotations();
            let mut targets = Targets::default();
            let n = targets_raw.len();
            for t in targets_raw.into_iter().flatten() {
                if t.value == "*" {
                    if n > 1 {
                        p.err(
                            t.span,
                            "Wildcard should not be specified together with other targets.",
                        );
                    }
                    targets = Targets::all();
                    continue;
                }
                let slot: Option<&mut bool> = match t.value.as_str() {
                    "file" => Some(&mut targets.file),
                    "const" => Some(&mut targets.constant),
                    "enum" => Some(&mut targets.enumeration),
                    "enumerant" => Some(&mut targets.enumerant),
                    "struct" => Some(&mut targets.structure),
                    "field" => Some(&mut targets.field),
                    "union" => Some(&mut targets.union),
                    "group" => Some(&mut targets.group),
                    "interface" => Some(&mut targets.interface),
                    "method" => Some(&mut targets.method),
                    "param" => Some(&mut targets.param),
                    "annotation" => Some(&mut targets.annotation),
                    _ => None,
                };
                match slot {
                    Some(s) => {
                        if *s {
                            p.err(t.span, "Duplicate target specification.");
                        }
                        *s = true;
                    }
                    None => p.err(t.span, "Not a valid annotation target."),
                }
            }
            let mut d = Self::decl(
                name,
                id.map_or(DeclId::Unspecified, DeclId::Uid),
                DeclBody::Annotation { ty, targets },
            );
            d.annotations = annotations;
            Some(d)
        })
    }
}
