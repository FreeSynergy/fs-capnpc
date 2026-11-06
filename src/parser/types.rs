//! Parser state types: nesting level, parse results and the error context.
//!
//! Pattern: State — `Level` tracks which declaration kinds are legal; `Parsed` is the per-statement result.

use crate::ast::*;
use crate::error::Diagnostic;

/// Nesting level of the statement being parsed.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Level {
    File,
    Enum,
    Struct,
    Interface,
}

/// Per-file parse context collecting diagnostics.
pub(super) struct Ctx {
    pub(super) file: String,
    pub(super) errors: Vec<Diagnostic>,
}

/// Result of parsing one statement.
pub(super) enum Parsed {
    Decl(Declaration),
    NakedId(Located<u64>, Option<String>),
    NakedAnnotation(AnnotationApplication),
}
