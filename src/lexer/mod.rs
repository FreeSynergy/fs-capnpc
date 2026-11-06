//! Lexer (counterpart of `lexer.c++` / `lexer.capnp`).
//!
//! Turns schema text into a tree of [`Statement`]s. A statement is a flat list of [`Token`]s
//! terminated by `;` or by a `{ ... }` block of nested statements. Parenthesised and bracketed
//! groups are lexed into nested comma-separated token lists, exactly like the official lexer.
//! Doc comments are the `#` comment lines that follow a `;` or `{` (or a closing `}`).
//!
//! Pattern: Facade — this module only re-exports the lexer entry points and data types.

mod cursor;
mod entry;
mod literals;
mod statements;
mod tokens;
mod types;

#[cfg(test)]
mod tests;

pub use entry::{lex, lex_tokens};
pub use types::{Body, Statement, Tok, Token};

const MAX_NESTING_DEPTH: u32 = 64;
