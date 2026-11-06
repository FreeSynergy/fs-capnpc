//! Statement-level lexing: statement sequences, blocks and `;` terminators.
//!
//! Pattern: Template method — each statement is tokens followed by `;` or a `{ ... }` block.

use super::MAX_NESTING_DEPTH;
use super::cursor::Lexer;
use super::types::{Body, Statement};
use crate::error::Span;

impl Lexer<'_> {
    pub(super) fn statement_sequence(&mut self) -> Vec<Statement> {
        let mut out = vec![];
        self.skip_comments_and_ws();
        while let Some(s) = self.statement() {
            out.push(s);
            self.skip_comments_and_ws();
        }
        out
    }

    pub(super) fn statement(&mut self) -> Option<Statement> {
        if self.depth >= MAX_NESTING_DEPTH {
            self.note(self.pos);
            return None;
        }
        self.depth += 1;
        let r = self.statement_inner();
        self.depth -= 1;
        r
    }

    pub(super) fn statement_inner(&mut self) -> Option<Statement> {
        let start = self.pos;
        let tokens = self.token_sequence();
        match self.peek() {
            Some(b';') => {
                self.pos += 1;
                let doc = self.doc_comment();
                Some(Statement {
                    tokens,
                    body: Body::Line,
                    doc_comment: doc,
                    span: Span::new(start as u32, self.pos as u32),
                })
            }
            Some(b'{') => {
                self.pos += 1;
                let early = self.doc_comment();
                let stmts = self.statement_sequence();
                if self.peek() != Some(b'}') {
                    self.note(self.pos);
                    self.pos = start;
                    return None;
                }
                self.pos += 1;
                let late = self.doc_comment();
                Some(Statement {
                    tokens,
                    body: Body::Block(stmts),
                    doc_comment: early.or(late),
                    span: Span::new(start as u32, self.pos as u32),
                })
            }
            _ => {
                self.note(self.pos);
                self.pos = start;
                None
            }
        }
    }
}
