//! Token and statement data types produced by the lexer.
//!
//! Pattern: Newtype — thin value types (`Tok`, `Token`, `Statement`) shared by lexer and parser.

use crate::error::Span;

/// A lexical token kind.
#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    /// Identifier or keyword.
    Ident(String),
    /// Text literal bytes.
    Str(Vec<u8>),
    /// Binary literal bytes.
    Bin(Vec<u8>),
    /// Integer literal magnitude.
    Int(u64),
    /// Floating-point literal.
    Float(f64),
    /// Operator or punctuation.
    Op(String),
    /// Parenthesised, comma-separated token groups.
    Paren(Vec<Vec<Token>>),
    /// Bracketed, comma-separated token groups.
    Bracket(Vec<Vec<Token>>),
}

/// A token with its source span.
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    /// Token kind and payload.
    pub tok: Tok,
    /// Source span.
    pub span: Span,
}

/// How a statement is terminated.
#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    /// Statement ends at its terminating semicolon.
    Line,
    /// Statement owns a braced block of nested statements.
    Block(Vec<Statement>),
}

/// A semicolon- or block-terminated run of tokens.
#[derive(Clone, Debug, PartialEq)]
pub struct Statement {
    /// Tokens before the body.
    pub tokens: Vec<Token>,
    /// Terminator of the statement.
    pub body: Body,
    /// Doc comment preceding the statement.
    pub doc_comment: Option<String>,
    /// Source span.
    pub span: Span,
}
