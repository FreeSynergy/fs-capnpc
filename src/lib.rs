//! `capnp-compiler`: a Cap'n Proto schema compiler in pure Rust.

pub mod ast;
pub mod error;
pub mod ids;
pub mod layout;
pub mod lexer;
pub mod schema;
pub mod source;
pub mod parser;
#[allow(clippy::all, clippy::pedantic)]
#[rustfmt::skip]
pub mod schema_capnp;
