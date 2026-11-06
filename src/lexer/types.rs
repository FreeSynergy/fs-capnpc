//! Token and statement data types produced by the lexer.
//!
//! Pattern: Newtype — thin value types (`Tok`, `Token`, `Statement`) shared by lexer and parser.

use crate::error::Span;

#[derive(Clone, Debug, PartialEq)]
/// A lexical token kind.
pub enum Tok {
    Ident(String),
    Str(Vec<u8>),
    Bin(Vec<u8>),
    Int(u64),
    Float(f64),
    Op(String),
    Paren(Vec<Vec<Token>>),
    Bracket(Vec<Vec<Token>>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    Line,
    Block(Vec<Statement>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Statement {
    pub tokens: Vec<Token>,
    pub body: Body,
    pub doc_comment: Option<String>,
    pub span: Span,
}
