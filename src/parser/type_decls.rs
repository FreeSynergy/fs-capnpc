//! Type-level declarations: using, const, enum, enumerant, struct, interface.
//!
//! Pattern: Strategy — one parser per declaration kind.

use super::cursor::{P, dummy_span};
use crate::ast::*;

impl<'a> P<'a> {
    pub(super) fn using_decl(&mut self) -> Option<Declaration> {
        self.attempt(|p| {
            p.keyword("using")?;
            let name = p.attempt(|p| {
                let n = p.ident()?;
                p.op("=")?;
                Some(n)
            });
            let target = p.expression()?;
            let name = match name {
                Some(n) => n,
                None => {
                    if let ExprKind::Member { name, .. } = &target.kind {
                        name.clone()
                    } else {
                        p.err(
                            target.span,
                            "'using' declaration without '=' must specify a named declaration from a different scope.",
                        );
                        Located::new(String::new(), dummy_span())
                    }
                }
            };
            Some(Self::decl(name, DeclId::Unspecified, DeclBody::Using { target }))
        })
    }

    pub(super) fn const_decl(&mut self) -> Option<Declaration> {
        self.attempt(|p| {
            p.keyword("const")?;
            let name = p.ident()?;
            let id = p.uid();
            p.op(":")?;
            let ty = p.expression()?;
            p.op("=")?;
            let value = p.expression()?;
            let annotations = p.annotations();
            let mut d = Self::decl(
                name,
                id.map(DeclId::Uid).unwrap_or(DeclId::Unspecified),
                DeclBody::Const { ty, value },
            );
            d.annotations = annotations;
            Some(d)
        })
    }

    pub(super) fn enum_decl(&mut self) -> Option<Declaration> {
        self.attempt(|p| {
            p.keyword("enum")?;
            let name = p.ident()?;
            let id = p.uid();
            let annotations = p.annotations();
            let mut d = Self::decl(
                name,
                id.map(DeclId::Uid).unwrap_or(DeclId::Unspecified),
                DeclBody::Enum,
            );
            d.annotations = annotations;
            Some(d)
        })
    }

    pub(super) fn enumerant_decl(&mut self) -> Option<Declaration> {
        self.attempt(|p| {
            let name = p.ident()?;
            let ord = p.ordinal()?;
            let annotations = p.annotations();
            let mut d = Self::decl(name, DeclId::Ordinal(ord), DeclBody::Enumerant);
            d.annotations = annotations;
            Some(d)
        })
    }

    pub(super) fn struct_decl(&mut self) -> Option<Declaration> {
        self.attempt(|p| {
            p.keyword("struct")?;
            let name = p.ident()?;
            let id = p.uid();
            let params = p.generic_params();
            let annotations = p.annotations();
            let mut d = Self::decl(
                name,
                id.map(DeclId::Uid).unwrap_or(DeclId::Unspecified),
                DeclBody::Struct,
            );
            d.parameters = params;
            d.annotations = annotations;
            Some(d)
        })
    }

    pub(super) fn interface_decl(&mut self) -> Option<Declaration> {
        self.attempt(|p| {
            p.keyword("interface")?;
            let name = p.ident()?;
            let id = p.uid();
            let params = p.generic_params();
            let supers = p.attempt(|p| {
                p.keyword("extends")?;
                let (items, span) = p.paren_items()?;
                Some(p.list_items(items, span, |p| p.expression()))
            });
            let annotations = p.annotations();
            let mut d = Self::decl(
                name,
                id.map(DeclId::Uid).unwrap_or(DeclId::Unspecified),
                DeclBody::Interface {
                    superclasses: supers.unwrap_or_default().into_iter().flatten().collect(),
                },
            );
            d.parameters = params;
            d.annotations = annotations;
            Some(d)
        })
    }
}
