//! Lexer cursor: character classes, whitespace/comment skipping, doc comments.
//!
//! Pattern: Iterator — a byte cursor (`pos`) advanced by the grammar rules in sibling files.

use crate::error::Diagnostic;

pub(super) struct Lexer<'a> {
    pub(super) src: &'a [u8],
    pub(super) pos: usize,
    pub(super) depth: u32,
    pub(super) best: usize,
    pub(super) errors: Vec<Diagnostic>,
}

pub(super) fn is_ws(c: u8) -> bool {
    matches!(c, b' ' | 0x0c | b'\n' | b'\r' | b'\t' | 0x0b)
}
pub(super) fn is_name_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}
pub(super) fn is_name_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}
pub(super) fn is_op_char(c: u8) -> bool {
    b"!$%&*+-./:<=>?@^|~".contains(&c)
}

pub(super) fn hex_val(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        _ => c - b'A' + 10,
    }
}

impl<'a> Lexer<'a> {
    pub(super) fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }
    pub(super) fn peek_at(&self, off: usize) -> Option<u8> {
        self.src.get(self.pos + off).copied()
    }
    pub(super) fn note(&mut self, p: usize) {
        if p > self.best {
            self.best = p;
        }
    }

    pub(super) fn skip_ws(&mut self) {
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

    pub(super) fn skip_comments_and_ws(&mut self) {
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
    pub(super) fn doc_comment(&mut self) -> Option<String> {
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

    pub(super) fn skip_line_ws(&mut self) {
        while self
            .peek()
            .is_some_and(|c| is_ws(c) && c != b'\n' && c != b'\r')
        {
            self.pos += 1;
        }
    }
}
