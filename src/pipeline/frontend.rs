//! Composition of a lex stage and a parse stage.
//!
//! Pattern: Facade over two Strategies — one object behind the [`Frontend`] interface.

use super::stages::{Frontend, LexStage, ParseStage, StdLexer, StdParser};
use crate::ast::Declaration;
use crate::error::Diagnostic;
use std::rc::Rc;

/// Front end built from any lexer and parser.
pub struct Staged<L, P> {
    lexer: L,
    parser: P,
}

impl<L: LexStage, P: ParseStage> Staged<L, P> {
    /// Compose the given stages.
    #[must_use]
    pub fn new(lexer: L, parser: P) -> Self {
        Staged { lexer, parser }
    }
}

/// The standard lexer and parser.
pub type DefaultFrontend = Staged<StdLexer, StdParser>;

impl DefaultFrontend {
    /// The standard lexer plus the standard parser.
    #[must_use]
    pub fn standard() -> Self {
        Staged::new(StdLexer, StdParser)
    }
}

impl<L: LexStage, P: ParseStage> Frontend for Staged<L, P> {
    fn parse_source(&self, source: &[u8], file: &str) -> (Rc<Declaration>, Vec<Diagnostic>) {
        match self.lexer.lex(source, file) {
            Ok(stmts) => {
                let (parsed, errs) = self.parser.parse(&stmts, file, true);
                (parsed.root, errs)
            }
            Err(errs) => {
                let (parsed, _) = self.parser.parse(&[], file, false);
                (parsed.root, errs)
            }
        }
    }
}
