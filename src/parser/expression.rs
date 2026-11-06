//! Expression parsing (types, values, tuples, lists, member access).
//!
//! Pattern: Template method — `expression` handles suffixes around `base_expression`.

use super::cursor::{P, dummy_span};
use crate::ast::*;
use crate::error::Span;
use crate::lexer::{Tok, Token};

impl<'a> P<'a> {
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

    pub(super) fn base_expression(&mut self) -> Option<Expression> {
        let t = self.peek()?;
        let span = t.span;
        match &t.tok {
            Tok::Int(v) => {
                self.pos += 1;
                Some(Expression {
                    kind: ExprKind::PositiveInt(*v),
                    span,
                })
            }
            Tok::Float(v) => {
                self.pos += 1;
                Some(Expression {
                    kind: ExprKind::Float(*v),
                    span,
                })
            }
            Tok::Op(o) if o == "-" => {
                self.pos += 1;
                let t2 = self.peek()?;
                match &t2.tok {
                    Tok::Int(v) => {
                        self.pos += 1;
                        Some(Expression {
                            kind: ExprKind::NegativeInt(*v),
                            span: t2.span,
                        })
                    }
                    Tok::Float(v) => {
                        self.pos += 1;
                        Some(Expression {
                            kind: ExprKind::Float(-*v),
                            span: t2.span,
                        })
                    }
                    Tok::Ident(s) if s == "inf" => {
                        self.pos += 1;
                        Some(Expression {
                            kind: ExprKind::Float(f64::NEG_INFINITY),
                            span: Span::new(span.start, t2.span.end),
                        })
                    }
                    _ => {
                        self.pos -= 1;
                        None
                    }
                }
            }
            Tok::Op(o) if o == "." => {
                self.pos += 1;
                match self.ident() {
                    Some(n) => Some(Expression {
                        kind: ExprKind::AbsoluteName(n.value),
                        span: Span::new(span.start, n.span.end),
                    }),
                    None => {
                        self.pos -= 1;
                        None
                    }
                }
            }
            Tok::Str(_) => {
                let mut out = vec![];
                let mut end = span.end;
                while let Some(t) = self.peek() {
                    if let Tok::Str(s) = &t.tok {
                        out.extend_from_slice(s);
                        end = t.span.end;
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
                Some(Expression {
                    kind: ExprKind::String(out),
                    span: Span::new(span.start, end),
                })
            }
            Tok::Bin(d) => {
                self.pos += 1;
                Some(Expression {
                    kind: ExprKind::Binary(d.clone()),
                    span,
                })
            }
            Tok::Bracket(items) => {
                self.pos += 1;
                let parsed = self.list_items(items, span, |p| p.expression());
                let list = parsed.into_iter().flatten().collect();
                Some(Expression {
                    kind: ExprKind::List(list),
                    span,
                })
            }
            Tok::Paren(items) => {
                self.pos += 1;
                let mut params = self.tuple_params(items, span);
                if params.len() == 1 && params[0].name.is_none() {
                    Some(params.pop().unwrap().value)
                } else {
                    Some(Expression {
                        kind: ExprKind::Tuple(params),
                        span,
                    })
                }
            }
            Tok::Ident(s) if s == "import" || s == "embed" => {
                let is_import = s == "import";
                let save = self.pos;
                self.pos += 1;
                if let Some(t2) = self.peek() {
                    if let Tok::Str(bytes) = &t2.tok {
                        self.pos += 1;
                        let v = String::from_utf8_lossy(bytes).into_owned();
                        return Some(Expression {
                            kind: if is_import {
                                ExprKind::Import(v)
                            } else {
                                ExprKind::Embed(v)
                            },
                            span: Span::new(span.start, t2.span.end),
                        });
                    }
                }
                self.pos = save;
                self.pos += 1;
                Some(Expression {
                    kind: ExprKind::RelativeName(s.clone()),
                    span,
                })
            }
            Tok::Ident(s) => {
                self.pos += 1;
                Some(Expression {
                    kind: ExprKind::RelativeName(s.clone()),
                    span,
                })
            }
            _ => None,
        }
    }
}
