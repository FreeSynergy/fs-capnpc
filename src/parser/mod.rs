//! Parser (counterpart of `parser.c++`): statement/token tree -> [`Declaration`](crate::ast::Declaration) AST.
//!
//! Alternatives are tried in the same order as the official parser (the first alternative that
//! matches a prefix wins, then the statement must be fully consumed).
//!
//! Pattern: Facade — this module only declares sub-modules and re-exports the public API.

mod annotation;
mod base_expression;
mod cursor;
mod declaration;
mod expression;
mod file;
mod members;
mod methods;
mod render;
mod statement;
#[cfg(test)]
mod tests;
mod type_decls;
mod types;

pub use file::{ParsedFile, parse_file};
pub use render::expression_string;
