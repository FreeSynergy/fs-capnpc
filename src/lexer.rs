//! Lexer (counterpart of `lexer.c++` / `lexer.capnp`).
//!
//! Turns schema text into a tree of [`Statement`]s. A statement is a flat list of [`Token`]s
//! terminated by `;` or by a `{ ... }` block of nested statements. Parenthesised and bracketed
//! groups are lexed into nested comma-separated token lists, exactly like the official lexer.
//! Doc comments are the `#` comment lines that follow a `;` or `{` (or a closing `}`).

use crate::error::{Diagnostic, Span};

const MAX_NESTING_DEPTH: u32 = 64;

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Ident(String),
    Str(Vec<u8>),
    Bin(Vec<u8>),
    Int(u64),
    Float(f64),
    Op(String),
    Paren(Vec<Vec<Token>>),
    Bracket(Vec<Vec<Token>>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    Line,
    Block(Vec<Statement>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Statement {
    pub tokens: Vec<Token>,
    pub body: Body,
    pub doc_comment: Option<String>,
    pub span: Span,
}

/// Lex a whole file into statements.
pub fn lex(src: &[u8], file: &str) -> Result<Vec<Statement>, Vec<Diagnostic>> {
    let mut lx = Lexer { src, pos: 0, depth: 0, best: 0, errors: vec![] };
    let out = lx.statement_sequence();
    lx.skip_comments_and_ws();
    if lx.pos == src.len() && lx.errors.is_empty() {
        return Ok(out);
    }
    let mut errors = std::mem::take(&mut lx.errors);
    if lx.pos != src.len() || errors.is_empty() {
        let best = lx.best.max(lx.pos) as u32;
        errors.push(Diagnostic {
            file: file.to_string(),
            span: Span::new(best, best),
            message: "Parse error.".into(),
        });
    }
    for e in &mut errors {
        e.file = file.to_string();
    }
    Err(errors)
}

/// Lex a token sequence (no statements); used by tests.
pub fn lex_tokens(src: &[u8]) -> Result<Vec<Token>, Vec<Diagnostic>> {
    let mut lx = Lexer { src, pos: 0, depth: 0, best: 0, errors: vec![] };
    let toks = lx.token_sequence();
    if lx.pos == src.len() && lx.errors.is_empty() {
        Ok(toks)
    } else {
        let best = lx.best.max(lx.pos) as u32;
        Err(vec![Diagnostic { file: String::new(), span: Span::new(best, best), message: "Parse error.".into() }])
    }
}

struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    depth: u32,
    best: usize,
    errors: Vec<Diagnostic>,
}

fn is_ws(c: u8) -> bool {
    matches!(c, b' ' | 0x0c | b'\n' | b'\r' | b'\t' | 0x0b)
}
fn is_name_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}
fn is_name_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}
fn is_op_char(c: u8) -> bool {
    b"!$%&*+-./:<=>?@^|~".contains(&c)
}

fn hex_val(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        _ => c - b'A' + 10,
    }
}

impl<'a> Lexer<'a> {
    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }
    fn peek_at(&self, off: usize) -> Option<u8> {
        self.src.get(self.pos + off).copied()
    }
    fn note(&mut self, p: usize) {
        if p > self.best {
            self.best = p;
        }
    }

    fn skip_ws(&mut self) {
        loop {
            while self.peek().is_some_and(is_ws) {
                self.pos += 1;
            }
            if self.src[self.pos..].starts_with(&[0xef, 0xbb, 0xbf]) {
                self.pos += 3;
            } else {
                break;
            }
        }
    }

    fn skip_comments_and_ws(&mut self) {
        loop {
            self.skip_ws();
            if self.peek() == Some(b'#') {
                while self.peek().is_some_and(|c| c != b'\n') {
                    self.pos += 1;
                }
                if self.peek() == Some(b'\n') {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    /// Doc comment: comment lines preceded by at most one newline and no blank lines.
    fn doc_comment(&mut self) -> Option<String> {
        let save = self.pos;
        self.skip_line_ws();
        // optional newline
        match self.peek() {
            Some(b'\n') => self.pos += 1,
            Some(b'\r') => {
                self.pos += 1;
                if self.peek() == Some(b'\n') {
                    self.pos += 1;
                }
            }
            _ => {}
        }
        let mut out = String::new();
        let mut count = 0;
        loop {
            let before = self.pos;
            self.skip_line_ws();
            if self.peek() != Some(b'#') {
                self.pos = before;
                break;
            }
            self.pos += 1;
            if self.peek() == Some(b' ') {
                self.pos += 1;
            }
            let start = self.pos;
            while self.peek().is_some_and(|c| c != b'\n') {
                self.pos += 1;
            }
            out.push_str(&String::from_utf8_lossy(&self.src[start..self.pos]));
            out.push('\n');
            if self.peek() == Some(b'\n') {
                self.pos += 1;
            }
            count += 1;
        }
        if count == 0 {
            self.pos = save;
            None
        } else {
            Some(out)
        }
    }

    fn skip_line_ws(&mut self) {
        while self.peek().is_some_and(|c| is_ws(c) && c != b'\n' && c != b'\r') {
            self.pos += 1;
        }
    }

    fn statement_sequence(&mut self) -> Vec<Statement> {
        let mut out = vec![];
        self.skip_comments_and_ws();
        while let Some(s) = self.statement() {
            out.push(s);
            self.skip_comments_and_ws();
        }
        out
    }

    fn statement(&mut self) -> Option<Statement> {
        if self.depth >= MAX_NESTING_DEPTH {
            self.note(self.pos);
            return None;
        }
        self.depth += 1;
        let r = self.statement_inner();
        self.depth -= 1;
        r
    }

    fn statement_inner(&mut self) -> Option<Statement> {
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

    fn token_sequence(&mut self) -> Vec<Token> {
        let mut out = vec![];
        self.skip_comments_and_ws();
        while let Some(t) = self.token() {
            out.push(t);
            self.skip_comments_and_ws();
        }
        out
    }

    fn token(&mut self) -> Option<Token> {
        if self.depth >= MAX_NESTING_DEPTH {
            self.note(self.pos);
            return None;
        }
        self.depth += 1;
        let r = self.raw_token();
        self.depth -= 1;
        r
    }

    fn raw_token(&mut self) -> Option<Token> {
        let start = self.pos;
        let c = self.peek()?;
        let mk = |tok: Tok, end: usize| Token { tok, span: Span::new(start as u32, end as u32) };

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
                let tok = if c == b'(' { Tok::Paren(items) } else { Tok::Bracket(items) };
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
                message: "Non-UTF-8 input detected. Cap'n Proto schema files must be UTF-8 text.".into(),
            });
            self.pos = start + len;
            return None;
        }
        self.note(start);
        None
    }

    fn comma_list(&mut self) -> Vec<Vec<Token>> {
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

    fn quoted_string(&mut self) -> Option<Vec<u8>> {
        debug_assert_eq!(self.peek(), Some(b'"'));
        self.pos += 1;
        let mut out = vec![];
        loop {
            let c = self.peek()?;
            match c {
                b'"' => {
                    self.pos += 1;
                    return Some(out);
                }
                b'\n' => return None,
                b'\\' => {
                    self.pos += 1;
                    let e = self.peek()?;
                    match e {
                        b'a' => out.push(7),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'v' => out.push(11),
                        b'\'' | b'"' | b'\\' | b'?' => out.push(e),
                        b'x' => {
                            let h1 = self.peek_at(1).filter(|c| c.is_ascii_hexdigit())?;
                            let h2 = self.peek_at(2).filter(|c| c.is_ascii_hexdigit())?;
                            out.push((hex_val(h1) << 4) | hex_val(h2));
                            self.pos += 2;
                        }
                        b'0'..=b'7' => {
                            let mut v = (e - b'0') as u32;
                            if let Some(d) = self.peek_at(1).filter(|c| (b'0'..=b'7').contains(c)) {
                                v = (v << 3) | (d - b'0') as u32;
                                self.pos += 1;
                                if let Some(d) = self.peek_at(1).filter(|c| (b'0'..=b'7').contains(c)) {
                                    v = (v << 3) | (d - b'0') as u32;
                                    self.pos += 1;
                                }
                            }
                            out.push(v as u8);
                        }
                        _ => return None,
                    }
                    self.pos += 1;
                }
                _ => {
                    out.push(c);
                    self.pos += 1;
                }
            }
        }
    }

    fn hex_binary(&mut self) -> Option<Vec<u8>> {
        self.pos += 3; // 0x"
        let mut out = vec![];
        loop {
            self.skip_ws();
            let h1 = self.peek().filter(|c| c.is_ascii_hexdigit());
            let h2 = self.peek_at(1).filter(|c| c.is_ascii_hexdigit());
            match (h1, h2) {
                (Some(a), Some(b)) => {
                    out.push((hex_val(a) << 4) | hex_val(b));
                    self.pos += 2;
                }
                _ => break,
            }
        }
        if out.is_empty() {
            return None;
        }
        if self.peek() == Some(b'"') {
            self.pos += 1;
            Some(out)
        } else {
            None
        }
    }

    fn not_looking_at_alpha(&self) -> bool {
        !self.peek().is_some_and(|c| c.is_ascii_alphabetic() || c == b'_' || c == b'.')
    }

    fn integer(&mut self) -> Option<u64> {
        let start = self.pos;
        let c = self.peek()?;
        let v: u64;
        if c == b'0' && self.peek_at(1) == Some(b'x') && self.peek_at(2).is_some_and(|c| c.is_ascii_hexdigit()) {
            self.pos += 2;
            let mut r: u64 = 0;
            while let Some(d) = self.peek().filter(|c| c.is_ascii_hexdigit()) {
                r = r.wrapping_mul(16).wrapping_add(hex_val(d) as u64);
                self.pos += 1;
            }
            v = r;
        } else if c == b'0' {
            self.pos += 1;
            let mut r: u64 = 0;
            while let Some(d) = self.peek().filter(|c| (b'0'..=b'7').contains(c)) {
                r = r.wrapping_mul(8).wrapping_add((d - b'0') as u64);
                self.pos += 1;
            }
            v = r;
        } else {
            let mut r: u64 = 0;
            while let Some(d) = self.peek().filter(|c| c.is_ascii_digit()) {
                r = r.wrapping_mul(10).wrapping_add((d - b'0') as u64);
                self.pos += 1;
            }
            v = r;
        }
        if self.not_looking_at_alpha() {
            Some(v)
        } else {
            self.pos = start;
            None
        }
    }

    fn number(&mut self) -> Option<f64> {
        let start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.pos += 1;
        }
        let int_part = String::from_utf8_lossy(&self.src[start..self.pos]).into_owned();
        let mut frac = String::new();
        if self.peek() == Some(b'.') {
            self.pos += 1;
            let s = self.pos;
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.pos += 1;
            }
            frac = String::from_utf8_lossy(&self.src[s..self.pos]).into_owned();
        }
        let mut exp = String::new();
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos += 1;
            if let Some(sg @ (b'+' | b'-')) = self.peek() {
                exp.push(sg as char);
                self.pos += 1;
            }
            let s = self.pos;
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.pos += 1;
            }
            exp.push_str(&String::from_utf8_lossy(&self.src[s..self.pos]));
        }
        if !self.not_looking_at_alpha() {
            self.pos = start;
            return None;
        }
        let mut text = int_part;
        if !frac.is_empty() {
            text.push('.');
            text.push_str(&frac);
        }
        if exp.chars().any(|c| c.is_ascii_digit()) {
            text.push('e');
            text.push_str(&exp);
        }
        text.parse::<f64>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<Tok> {
        lex_tokens(src.as_bytes()).unwrap().into_iter().map(|t| t.tok).collect()
    }

    #[test]
    fn identifiers_and_ops() {
        assert_eq!(
            kinds("foo @0x1f :Int32"),
            vec![
                Tok::Ident("foo".into()),
                Tok::Op("@".into()),
                Tok::Int(0x1f),
                Tok::Op(":".into()),
                Tok::Ident("Int32".into())
            ]
        );
    }

    #[test]
    fn numbers() {
        assert_eq!(kinds("12 0 017 0x10 1.5 2e3 1e+2"), vec![
            Tok::Int(12), Tok::Int(0), Tok::Int(0o17), Tok::Int(16),
            Tok::Float(1.5), Tok::Float(2000.0), Tok::Float(100.0)
        ]);
    }

    #[test]
    fn strings() {
        assert_eq!(kinds(r#""a\n\x41\101\"""#), vec![Tok::Str(b"a\nAA\"".to_vec())]);
        assert_eq!(kinds("`raw text\n"), vec![Tok::Str(b"raw text\n".to_vec())]);
        assert_eq!(kinds(r#"0x"01 ab ff""#), vec![Tok::Bin(vec![1, 0xab, 0xff])]);
    }

    #[test]
    fn lists() {
        let t = lex_tokens(b"(a, b = 1,) [x]").unwrap();
        match &t[0].tok {
            Tok::Paren(items) => assert_eq!(items.len(), 2),
            _ => panic!(),
        }
        match &t[1].tok {
            Tok::Bracket(items) => assert_eq!(items.len(), 1),
            _ => panic!(),
        }
        match &lex_tokens(b"()").unwrap()[0].tok {
            Tok::Paren(items) => assert!(items.is_empty()),
            _ => panic!(),
        }
    }

    #[test]
    fn statements_and_doc_comments() {
        let src = "# ignored\nstruct Foo {  # doc for Foo\n  # more\n  a @0 :Int32;  # doc a\n  b @1 :Int8;\n}\n";
        let st = lex(src.as_bytes(), "x").unwrap();
        assert_eq!(st.len(), 1);
        assert_eq!(st[0].doc_comment.as_deref(), Some("doc for Foo\nmore\n"));
        match &st[0].body {
            Body::Block(b) => {
                assert_eq!(b.len(), 2);
                assert_eq!(b[0].doc_comment.as_deref(), Some("doc a\n"));
                assert_eq!(b[1].doc_comment, None);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn trailing_block_comment() {
        let st = lex(b"struct A {\n a @0 :Int8;\n}\n# late doc\n", "x").unwrap();
        assert_eq!(st[0].doc_comment.as_deref(), Some("late doc\n"));
    }

    #[test]
    fn errors() {
        assert!(lex(b"struct A {", "x").is_err());
        assert!(lex(b"foo bar", "x").is_err());
        assert!(lex(b"\"unterminated\n;", "x").is_err());
    }
}
