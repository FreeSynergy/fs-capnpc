//! Expression parsing (types, values, tuples, lists, member access).
//!
//! Pattern: Template method — `expression` handles suffixes around `base_expression`.

use super::cursor::{P, dummy_span};
use crate::ast::*;
use crate::error::Span;
use crate::lexer::Token;

impl P<'_> {
    pub(super) fn expression(&mut self) -> Option<Expression> {
        let mut base = self.base_expression()?;
        loop {
            let start = base.span.start;
            let suffix = self.attempt(|p| {
                if let Some(dot) = p.op(".") {
                    let name = p.ident()?;
                    return Some((Some(name), None, Span::new(dot.start, 0)));
                }
                let (items, span) = p.paren_items()?;
                Some((None, Some(p.tuple_params(items, span)), span))
            });
            match suffix {
                Some((Some(name), _, _)) => {
                    let end = name.span.end;
                    base = Expression {
                        kind: ExprKind::Member {
                            parent: Box::new(base),
                            name,
                        },
                        span: Span::new(start, end),
                    };
                }
                Some((None, Some(params), span)) => {
                    base = Expression {
                        kind: ExprKind::Application {
                            function: Box::new(base),
                            params,
                        },
                        span: Span::new(start, span.end),
                    };
                }
                _ => break,
            }
        }
        Some(base)
    }

    pub(super) fn tuple_params(&mut self, items: &[Vec<Token>], span: Span) -> Vec<ExprParam> {
        let parsed = self.list_items(items, span, |p| {
            let name = p.attempt(|p| {
                let n = p.ident()?;
                p.op("=")?;
                Some(n)
            });
            let value = p.expression()?;
            Some(ExprParam { name, value })
        });
        parsed
            .into_iter()
            .map(|e| {
                e.unwrap_or(ExprParam {
                    name: None,
                    value: Expression {
                        kind: ExprKind::Unknown,
                        span: dummy_span(),
                    },
                })
            })
            .collect()
    }
}
