//! The error returned by a failed compilation.
//!
//! Pattern: Newtype-style error value — collects every diagnostic, rendered for display.

use super::diagnostic::Diagnostic;
use std::fmt;

/// Failure of a whole compilation: all collected diagnostics (rendered with source context).
#[derive(Debug)]
pub struct Error {
    /// Structured diagnostics, in report order.
    pub diagnostics: Vec<Diagnostic>,
    /// Diagnostics rendered with line and column, ready to print.
    pub rendered: Vec<String>,
}

impl Error {
    /// Error carrying one pre-rendered message and no structured diagnostics.
    pub fn single(message: impl Into<String>) -> Self {
        let m = message.into();
        Error {
            diagnostics: vec![],
            rendered: vec![m],
        }
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
