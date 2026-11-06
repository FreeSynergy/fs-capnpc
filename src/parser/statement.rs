//! Statement-level parsing: one statement plus its nested block.
//!
//! Pattern: Template method — parse the head, then validate the body against the declared member level.

use super::cursor::P;
use super::types::{Ctx, Level, Parsed};
use crate::error::{Diagnostic, Span};
use crate::lexer::{Body, Statement};

impl Ctx {
    pub(super) fn err(&mut self, span: Span, msg: impl Into<String>) {
        self.errors.push(Diagnostic {
            file: self.file.clone(),
            span,
            message: msg.into(),
        });
    }

    pub(super) fn parse_statement(&mut self, st: &Statement, level: Level) -> Option<Parsed> {
        let mut p = P::new(&st.tokens);
        let result = p.declaration(level);
        let ok = match &result {
            Some(_) => p.at_end(),
            None => false,
        };
        self.errors.extend(p.take_errors(&self.file));
        if !ok {
            let best = p.best;
            let byte = if best < st.tokens.len() {
                st.tokens[best].span.start
            } else if let Some(last) = st.tokens.last() {
                last.span.end
            } else {
                st.span.start
            };
            self.err(Span::new(byte, byte), "Parse error.");
            return None;
        }
        let (mut parsed, member_level) = result.unwrap();
        match &mut parsed {
            Parsed::Decl(d) => {
                d.doc_comment = st.doc_comment.clone();
                d.span = st.span;
            }
            Parsed::NakedId(_, doc) => *doc = st.doc_comment.clone(),
            Parsed::NakedAnnotation(_) => {}
        }
        match (&st.body, member_level) {
            (Body::Line, Some(_)) => {
                self.err(
                    st.span,
                    "This statement should end with a block, not a semicolon.",
                );
            }
            (Body::Block(members), Some(ml)) => {
                let mut nested = vec![];
                for m in members {
                    if let Some(Parsed::Decl(d)) = self.parse_statement(m, ml) {
                        nested.push(d);
                    }
                }
                if let Parsed::Decl(d) = &mut parsed {
                    d.nested = nested.into_iter().map(std::rc::Rc::new).collect();
                }
            }
            (Body::Block(_), None) => {
                self.err(
                    st.span,
                    "This statement should end with a semicolon, not a block.",
                );
            }
            (Body::Line, None) => {}
        }
        Some(parsed)
    }
}
