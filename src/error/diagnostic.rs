//! One diagnostic with line/column rendering.
//!
//! Pattern: Adapter — turns (file, span, message) plus source text into a printable line.

use super::span::Span;
use std::fmt;

/// One error message attached to a file and byte range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// Source file the message refers to.
    pub file: String,
    /// Byte range within the file.
    pub span: Span,
    /// Human-readable error text.
    pub message: String,
}

impl Diagnostic {
    /// Render as `file:line:col: error: message`, using the source text for line/column.
    pub fn render(&self, source: Option<&[u8]>) -> String {
        match source {
            Some(src) => {
                let (line, col) = line_col(src, self.span.start as usize);
                format!("{}:{}:{}: error: {}", self.file, line, col, self.message)
            }
            None => format!("{}: error: {}", self.file, self.message),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}-{}: error: {}",
            self.file, self.span.start, self.span.end, self.message
        )
    }
}

/// Convert a byte offset to 1-based (line, column).
pub fn line_col(src: &[u8], offset: usize) -> (usize, usize) {
    let offset = offset.min(src.len());
    let mut line = 1;
    let mut col = 1;
    for &b in &src[..offset] {
        if b == b'\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}
