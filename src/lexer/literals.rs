//! Literal lexing: quoted strings, hex binaries, integers and floats.
//!
//! Pattern: Template method — each literal class has its own scanning rule.

use super::cursor::{Lexer, hex_val};

impl Lexer<'_> {
    pub(super) fn quoted_string(&mut self) -> Option<Vec<u8>> {
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
                            let mut v = u32::from(e - b'0');
                            if let Some(d) = self.peek_at(1).filter(|c| (b'0'..=b'7').contains(c)) {
                                v = (v << 3) | u32::from(d - b'0');
                                self.pos += 1;
                                if let Some(d) =
                                    self.peek_at(1).filter(|c| (b'0'..=b'7').contains(c))
                                {
                                    v = (v << 3) | u32::from(d - b'0');
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

    pub(super) fn hex_binary(&mut self) -> Option<Vec<u8>> {
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

    pub(super) fn not_looking_at_alpha(&self) -> bool {
        !self
            .peek()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_' || c == b'.')
    }

    pub(super) fn integer(&mut self) -> Option<u64> {
        let start = self.pos;
        let c = self.peek()?;

        let v: u64 = if c == b'0'
            && self.peek_at(1) == Some(b'x')
            && self.peek_at(2).is_some_and(|c| c.is_ascii_hexdigit())
        {
            self.pos += 2;
            let mut r: u64 = 0;
            while let Some(d) = self.peek().filter(|c| c.is_ascii_hexdigit()) {
                r = r.wrapping_mul(16).wrapping_add(u64::from(hex_val(d)));
                self.pos += 1;
            }
            r
        } else if c == b'0' {
            self.pos += 1;
            let mut r: u64 = 0;
            while let Some(d) = self.peek().filter(|c| (b'0'..=b'7').contains(c)) {
                r = r.wrapping_mul(8).wrapping_add(u64::from(d - b'0'));
                self.pos += 1;
            }
            r
        } else {
            let mut r: u64 = 0;
            while let Some(d) = self.peek().filter(|c| c.is_ascii_digit()) {
                r = r.wrapping_mul(10).wrapping_add(u64::from(d - b'0'));
                self.pos += 1;
            }
            r
        };
        if self.not_looking_at_alpha() {
            Some(v)
        } else {
            self.pos = start;
            None
        }
    }

    pub(super) fn number(&mut self) -> Option<f64> {
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
