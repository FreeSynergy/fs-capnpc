//! Public lexer entry points.
//!
//! Pattern: Facade — `lex` / `lex_tokens` hide the internal cursor-based `Lexer`.

use super::cursor::Lexer;
use super::types::{Statement, Token};
use crate::error::{Diagnostic, Span};

/// Lex a whole file into statements.
pub fn lex(src: &[u8], file: &str) -> Result<Vec<Statement>, Vec<Diagnostic>> {
    let mut lx = Lexer {
        src,
        pos: 0,
        depth: 0,
        best: 0,
        errors: vec![],
    };
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
    let mut lx = Lexer {
        src,
        pos: 0,
        depth: 0,
        best: 0,
        errors: vec![],
    };
    let toks = lx.token_sequence();
    if lx.pos == src.len() && lx.errors.is_empty() {
        Ok(toks)
    } else {
        let best = lx.best.max(lx.pos) as u32;
        Err(vec![Diagnostic {
            file: String::new(),
            span: Span::new(best, best),
            message: "Parse error.".into(),
        }])
    }
}
