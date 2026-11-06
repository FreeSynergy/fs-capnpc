//! Interface methods and their parameter lists.
//!
//! Pattern: Strategy — one parser per method component.

use super::cursor::P;
use crate::ast::*;
use crate::error::Span;

impl<'a> P<'a> {
    pub(super) fn param(&mut self) -> Option<Param> {
        let start = self.pos;
        let name = self.ident()?;
        self.op(":")?;
        let ty = self.expression()?;
        let default_value = self.attempt(|p| {
            p.op("=")?;
            p.expression()
        });
        let annotations = self.annotations();
        let span = Span::new(self.t[start].span.start, self.t[self.pos - 1].span.end);
        Some(Param {
            name,
            ty,
            annotations,
            default_value,
            span,
        })
    }

    pub(super) fn param_list(&mut self) -> Option<ParamList> {
        if let Some((items, span)) = self.attempt(|p| p.paren_items()) {
            let params = self.list_items(items, span, |p| p.param());
            return Some(ParamList {
                kind: ParamListKind::Named(params.into_iter().flatten().collect()),
                span,
            });
        }
        if let Some(span) = self.keyword("stream") {
            return Some(ParamList {
                kind: ParamListKind::Stream,
                span,
            });
        }
        let e = self.expression()?;
        let span = e.span;
        Some(ParamList {
            kind: ParamListKind::Type(e),
            span,
        })
    }

    pub(super) fn method_decl(&mut self) -> Option<Declaration> {
        self.attempt(|p| {
            let name = p.ident()?;
            let ord = p.ordinal()?;
            let generic = p.attempt(|p| p.bracket_items());
            let params = p.param_list()?;
            let results = p.attempt(|p| {
                p.op("->")?;
                p.param_list()
            });
            let annotations = p.annotations();
            let mut d = Self::decl(
                name,
                DeclId::Ordinal(ord),
                DeclBody::Method { params, results },
            );
            d.annotations = annotations;
            if let Some((items, span)) = generic {
                d.parameters = p.brand_params(items, span);
            }
            Some(d)
        })
    }
}
