//! Duplicate name / ordinal detection for declaration members.
//!
//! Pattern: Visitor — walks the member declarations and reports clashes to the compiler.

use crate::ast::*;
use crate::compiler::*;
use crate::error::Span;
use std::collections::BTreeMap;
use std::rc::Rc;

pub(crate) fn expression_target_name(e: &Expression) -> &str {
    match &e.kind {
        ExprKind::AbsoluteName(n) | ExprKind::RelativeName(n) => n,
        ExprKind::Application { function, .. } => expression_target_name(function),
        ExprKind::Member { name, .. } => &name.value,
        _ => "",
    }
}

pub(crate) fn check_duplicate_names(
    c: &mut Compiler,
    module: usize,
    nested: &[Rc<Declaration>],
    parent_kind: DeclKind,
    names: &mut BTreeMap<String, Span>,
) {
    for decl in nested {
        let name = &decl.name;
        let text = &name.value;
        let kind = decl_kind(decl);
        if let Some(prev) = names.get(text).copied() {
            if text.is_empty() && matches!(decl.body, DeclBody::Union) {
                c.add_error(
                    module,
                    name.span,
                    "An unnamed union is already defined in this scope.",
                );
                c.add_error(module, prev, "Previously defined here.");
            } else {
                c.add_error(
                    module,
                    name.span,
                    format!("'{text}' is already defined in this scope."),
                );
                c.add_error(module, prev, format!("'{text}' previously defined here."));
            }
        } else {
            names.insert(text.clone(), name.span);
        }

        let first = text.chars().next();
        match &decl.body {
            DeclBody::Using { target } => {
                let tn = expression_target_name(target);
                if tn.chars().next().is_some_and(|ch| ch.is_ascii_lowercase()) {
                    if first.is_some_and(|ch| !ch.is_ascii_lowercase()) {
                        c.add_error(
                            module,
                            name.span,
                            "Non-type names must begin with a lower-case letter.",
                        );
                    }
                } else if first.is_some_and(|ch| !ch.is_ascii_uppercase()) {
                    c.add_error(
                        module,
                        name.span,
                        "Type names must begin with a capital letter.",
                    );
                }
            }
            DeclBody::Enum | DeclBody::Struct | DeclBody::Interface { .. } => {
                if first.is_some_and(|ch| !ch.is_ascii_uppercase()) {
                    c.add_error(
                        module,
                        name.span,
                        "Type names must begin with a capital letter.",
                    );
                }
            }
            DeclBody::Const { .. }
            | DeclBody::Annotation { .. }
            | DeclBody::Enumerant
            | DeclBody::Method { .. }
            | DeclBody::Field { .. }
            | DeclBody::Union
            | DeclBody::Group
                if first.is_some_and(|ch| !ch.is_ascii_lowercase()) =>
            {
                c.add_error(
                    module,
                    name.span,
                    "Non-type names must begin with a lower-case letter.",
                );
            }
            _ => {}
        }
        if text.contains('_') {
            c.add_error(
                module,
                name.span,
                "Cap'n Proto declaration names should use camelCase and must not contain underscores. (Code generators may convert names to the appropriate style for the target language.)",
            );
        }

        match kind {
            DeclKind::Using
            | DeclKind::Const
            | DeclKind::Enum
            | DeclKind::Struct
            | DeclKind::Interface
            | DeclKind::Annotation => {
                if !matches!(
                    parent_kind,
                    DeclKind::File | DeclKind::Struct | DeclKind::Interface
                ) {
                    c.add_error(
                        module,
                        decl.span,
                        "This kind of declaration doesn't belong here.",
                    );
                }
            }
            DeclKind::Enumerant => {
                if parent_kind != DeclKind::Enum {
                    c.add_error(module, decl.span, "Enumerants can only appear in enums.");
                }
            }
            DeclKind::Method => {
                if parent_kind != DeclKind::Interface {
                    c.add_error(module, decl.span, "Methods can only appear in interfaces.");
                }
            }
            DeclKind::Field | DeclKind::Union | DeclKind::Group => {
                if !matches!(
                    parent_kind,
                    DeclKind::Struct | DeclKind::Union | DeclKind::Group
                ) {
                    c.add_error(
                        module,
                        decl.span,
                        "This declaration can only appear in structs.",
                    );
                }
                if text.is_empty() {
                    check_duplicate_names(c, module, &decl.nested, kind, names);
                } else {
                    check_duplicate_names(c, module, &decl.nested, kind, &mut BTreeMap::new());
                }
            }
            _ => c.add_error(
                module,
                decl.span,
                "This kind of declaration doesn't belong here.",
            ),
        }
    }
}

pub(crate) struct DupOrdinal {
    expected: u64,
    last: Option<Located<u64>>,
}

impl DupOrdinal {
    pub(crate) fn new() -> Self {
        DupOrdinal {
            expected: 0,
            last: None,
        }
    }
    pub(crate) fn check(&mut self, c: &mut Compiler, module: usize, ord: &Located<u64>) {
        match ord.value.cmp(&self.expected) {
            std::cmp::Ordering::Less => {
                c.add_error(module, ord.span, "Duplicate ordinal number.");
                if let Some(last) = self.last.take() {
                    c.add_error(
                        module,
                        last.span,
                        format!("Ordinal @{} originally used here.", last.value),
                    );
                }
            }
            std::cmp::Ordering::Greater => {
                c.add_error(
                    module,
                    ord.span,
                    format!(
                        "Skipped ordinal @{}.  Ordinals must be sequential with no holes.",
                        self.expected
                    ),
                );
                self.expected = ord.value + 1;
            }
            std::cmp::Ordering::Equal => {
                self.expected += 1;
                self.last = Some(ord.clone());
            }
        }
    }
}
