//! Token cursor `P` with backtracking and list helpers.
//!
//! Pattern: Iterator — a cursor over a token slice with save/restore (`attempt`).

use crate::ast::*;
use crate::error::{Diagnostic, Span};
use crate::lexer::{Tok, Token};

/// Token cursor.
pub(super) struct P<'a> {
    pub(super) t: &'a [Token],
    pub(super) pos: usize,
    pub(super) best: usize,
    pub(super) errors: Vec<(Span, String)>,
}

pub(super) fn dummy_span() -> Span {
    Span::default()
}

impl<'a> P<'a> {
    pub(super) fn new(t: &'a [Token]) -> Self {
        P {
            t,
            pos: 0,
            best: 0,
            errors: vec![],
        }
    }

    pub(super) fn take_errors(&mut self, file: &str) -> Vec<Diagnostic> {
        std::mem::take(&mut self.errors)
            .into_iter()
            .map(|(span, message)| Diagnostic {
                file: file.to_string(),
                span,
                message,
            })
            .collect()
    }

    pub(super) fn err(&mut self, span: Span, msg: impl Into<String>) {
        self.errors.push((span, msg.into()));
    }

    pub(super) fn at_end(&mut self) -> bool {
        self.touch();
        self.pos >= self.t.len()
    }

    pub(super) fn touch(&mut self) {
        if self.pos > self.best {
            self.best = self.pos;
        }
    }

    pub(super) fn peek(&mut self) -> Option<&'a Token> {
        self.touch();
        self.t.get(self.pos)
    }

    /// Run `f`; restore the position when it fails.
    pub(super) fn attempt<T>(&mut self, f: impl FnOnce(&mut Self) -> Option<T>) -> Option<T> {
        let save = self.pos;
        let r = f(self);
        if r.is_none() {
            self.pos = save;
        }
        r
    }

    /// Parse the tokens of `toks` as a complete item with `f`.
    pub(super) fn sub<T>(
        &mut self,
        toks: &[Token],
        f: impl FnOnce(&mut P) -> Option<T>,
    ) -> Result<T, (usize, bool)> {
        let mut sp = P::new(toks);
        let r = f(&mut sp);
        let ok = r.is_some() && sp.at_end();
        let best = sp.best;
        self.errors.append(&mut sp.errors);
        match r {
            Some(v) if ok => Ok(v),
            _ => Err((best, true)),
        }
    }

    pub(super) fn ident(&mut self) -> Option<Located<String>> {
        let t = self.peek()?;
        if let Tok::Ident(s) = &t.tok {
            self.pos += 1;
            Some(Located::new(s.clone(), t.span))
        } else {
            None
        }
    }

    pub(super) fn keyword(&mut self, kw: &str) -> Option<Span> {
        let t = self.peek()?;
        match &t.tok {
            Tok::Ident(s) if s == kw => {
                self.pos += 1;
                Some(t.span)
            }
            _ => None,
        }
    }

    pub(super) fn op(&mut self, o: &str) -> Option<Span> {
        let t = self.peek()?;
        match &t.tok {
            Tok::Op(s) if s == o => {
                self.pos += 1;
                Some(t.span)
            }
            _ => None,
        }
    }

    pub(super) fn int(&mut self) -> Option<Located<u64>> {
        let t = self.peek()?;
        if let Tok::Int(v) = &t.tok {
            self.pos += 1;
            Some(Located::new(*v, t.span))
        } else {
            None
        }
    }

    pub(super) fn paren_items(&mut self) -> Option<(&'a Vec<Vec<Token>>, Span)> {
        let t = self.peek()?;
        if let Tok::Paren(items) = &t.tok {
            self.pos += 1;
            Some((items, t.span))
        } else {
            None
        }
    }

    pub(super) fn bracket_items(&mut self) -> Option<(&'a Vec<Vec<Token>>, Span)> {
        let t = self.peek()?;
        if let Tok::Bracket(items) = &t.tok {
            self.pos += 1;
            Some((items, t.span))
        } else {
            None
        }
    }

    /// Parse every item of a list token with `f`. Failed items yield `None` and an error.
    pub(super) fn list_items<T>(
        &mut self,
        items: &[Vec<Token>],
        list_span: Span,
        mut f: impl FnMut(&mut P) -> Option<T>,
    ) -> Vec<Option<T>> {
        let mut out = vec![];
        for item in items {
            match self.sub(item, &mut f) {
                Ok(v) => out.push(Some(v)),
                Err((best, _)) => {
                    if best < item.len() {
                        self.err(
                            Span::new(item[best].span.start, item.last().unwrap().span.end),
                            "Parse error.",
                        );
                    } else if !item.is_empty() {
                        self.err(
                            Span::new(item[0].span.start, item.last().unwrap().span.end),
                            "Parse error.",
                        );
                    } else {
                        self.err(list_span, "Parse error: Empty list item.");
                    }
                    out.push(None);
                }
            }
        }
        out
    }

    // ---------------------------------------------------------------- expressions
}
