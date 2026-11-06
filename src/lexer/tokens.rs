//! Token-level lexing: identifiers, operators, groups and comma lists.
//!
//! Pattern: Template method — `token` dispatches to one rule per token class.

use super::MAX_NESTING_DEPTH;
use super::cursor::{Lexer, is_name_char, is_name_start, is_op_char};
use super::types::{Tok, Token};
use crate::error::{Diagnostic, Span};

impl<'a> Lexer<'a> {
    pub(super) fn token_sequence(&mut self) -> Vec<Token> {
        let mut out = vec![];
        self.skip_comments_and_ws();
        while let Some(t) = self.token() {
            out.push(t);
            self.skip_comments_and_ws();
        }
        out
    }

    pub(super) fn token(&mut self) -> Option<Token> {
        if self.depth >= MAX_NESTING_DEPTH {
            self.note(self.pos);
            return None;
        }
        self.depth += 1;
        let r = self.raw_token();
        self.depth -= 1;
        r
    }

    pub(super) fn raw_token(&mut self) -> Option<Token> {
        let start = self.pos;
        let c = self.peek()?;
        let mk = |tok: Tok, end: usize| Token {
            tok,
            span: Span::new(start as u32, end as u32),
        };

        if is_name_start(c) {
            while self.peek().is_some_and(is_name_char) {
                self.pos += 1;
            }
            let s = String::from_utf8_lossy(&self.src[start..self.pos]).into_owned();
            return Some(mk(Tok::Ident(s), self.pos));
        }
        if c == b'"' {
            if let Some(s) = self.quoted_string() {
                return Some(mk(Tok::Str(s), self.pos));
            }
            self.pos = start;
        }
        if c == b'`' {
            self.pos += 1;
            let s = self.pos;
            while self.peek().is_some_and(|c| c != b'\r' && c != b'\n') {
                self.pos += 1;
            }
            let mut text = self.src[s..self.pos].to_vec();
            text.push(b'\n');
            return Some(mk(Tok::Str(text), self.pos));
        }
        if c == b'0' && self.peek_at(1) == Some(b'x') && self.peek_at(2) == Some(b'"') {
            if let Some(d) = self.hex_binary() {
                return Some(mk(Tok::Bin(d), self.pos));
            }
            self.pos = start;
        }
        if c.is_ascii_digit() {
            if let Some(v) = self.integer() {
                return Some(mk(Tok::Int(v), self.pos));
            }
            self.pos = start;
            if let Some(v) = self.number() {
                return Some(mk(Tok::Float(v), self.pos));
            }
            self.pos = start;
        }
        if is_op_char(c) {
            while self.peek().is_some_and(is_op_char) {
                self.pos += 1;
            }
            let s = String::from_utf8_lossy(&self.src[start..self.pos]).into_owned();
            return Some(mk(Tok::Op(s), self.pos));
        }
        if c == b'(' || c == b'[' {
            self.pos += 1;
            let items = self.comma_list();
            let close = if c == b'(' { b')' } else { b']' };
            if self.peek() == Some(close) {
                self.pos += 1;
                let tok = if c == b'(' {
                    Tok::Paren(items)
                } else {
                    Tok::Bracket(items)
                };
                return Some(mk(tok, self.pos));
            }
            self.note(self.pos);
            self.pos = start;
            return None;
        }
        if self.src[start..].starts_with(&[0xff, 0xfe])
            || self.src[start..].starts_with(&[0xfe, 0xff])
            || c == 0
        {
            let len = if c == 0 { 1 } else { 2 };
            self.errors.push(Diagnostic {
                file: String::new(),
                span: Span::new(start as u32, (start + len) as u32),
                message: "Non-UTF-8 input detected. Cap'n Proto schema files must be UTF-8 text."
                    .into(),
            });
            self.pos = start + len;
            return None;
        }
        self.note(start);
        None
    }

    pub(super) fn comma_list(&mut self) -> Vec<Vec<Token>> {
        let first = self.token_sequence();
        let mut rest = vec![];
        while self.peek() == Some(b',') {
            self.pos += 1;
            rest.push(self.token_sequence());
        }
        if first.is_empty() && rest.is_empty() {
            return vec![];
        }
        let mut rest_size = rest.len();
        if rest_size > 0 && rest[rest_size - 1].is_empty() {
            rest_size -= 1;
        }
        let mut out = Vec::with_capacity(1 + rest_size);
        out.push(first);
        out.extend(rest.into_iter().take(rest_size));
        out
    }
}
