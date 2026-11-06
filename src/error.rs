//! Diagnostics (counterpart of `error-reporter.c++`).

use std::fmt;

/// A byte range inside one source file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: u32, end: u32) -> Self {
        Span { start, end }
    }
    /// Smallest span covering both.
    pub fn to(self, other: Span) -> Span {
        Span { start: self.start.min(other.start), end: self.end.max(other.end) }
    }
}

/// One error message attached to a file and byte range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub file: String,
    pub span: Span,
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
        write!(f, "{}:{}-{}: error: {}", self.file, self.span.start, self.span.end, self.message)
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

/// Failure of a whole compilation: all collected diagnostics (rendered with source context).
#[derive(Debug)]
pub struct Error {
    pub diagnostics: Vec<Diagnostic>,
    pub rendered: Vec<String>,
}

impl Error {
    pub fn single(message: impl Into<String>) -> Self {
        let m = message.into();
        Error { diagnostics: vec![], rendered: vec![m] }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, line) in self.rendered.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{line}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
