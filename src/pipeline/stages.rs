//! Stage interfaces and the standard stage implementations.
//!
//! Pattern: Strategy — callers see traits; the concrete lexer/parser are interchangeable.

use crate::ast::Declaration;
use crate::error::Diagnostic;
use crate::lexer::{self, Statement};
use crate::parser::{self, ParsedFile};
use std::rc::Rc;

/// Source bytes to statement tree.
pub trait LexStage {
    /// Tokenise `src` (named `file` in diagnostics).
    ///
    /// # Errors
    /// Returns the lexical diagnostics.
    fn lex(&self, src: &[u8], file: &str) -> Result<Vec<Statement>, Vec<Diagnostic>>;
}

/// Statement tree to AST.
pub trait ParseStage {
    /// Parse `statements`; `require_id` demands the leading file id.
    fn parse(
        &self,
        statements: &[Statement],
        file: &str,
        require_id: bool,
    ) -> (ParsedFile, Vec<Diagnostic>);
}

/// Whole front end: bytes to a parsed file declaration plus diagnostics.
pub trait Frontend {
    /// Lex and parse `source`.
    fn parse_source(&self, source: &[u8], file: &str) -> (Rc<Declaration>, Vec<Diagnostic>);
}

/// The standard lexer.
#[derive(Clone, Copy, Debug, Default)]
pub struct StdLexer;

impl LexStage for StdLexer {
    fn lex(&self, src: &[u8], file: &str) -> Result<Vec<Statement>, Vec<Diagnostic>> {
        lexer::lex(src, file)
    }
}

/// The standard parser.
#[derive(Clone, Copy, Debug, Default)]
pub struct StdParser;

impl ParseStage for StdParser {
    fn parse(
        &self,
        statements: &[Statement],
        file: &str,
        require_id: bool,
    ) -> (ParsedFile, Vec<Diagnostic>) {
        parser::parse_file(statements, file, require_id)
    }
}
