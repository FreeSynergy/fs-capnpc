//! `capnp-compiler`: a Cap'n Proto schema compiler in pure Rust.

pub mod ast;
pub mod compiler;
pub mod error;
pub mod generics;
pub mod hashorder;
pub mod ids;
pub mod layout;
pub mod lexer;
pub mod parser;
pub mod request;
pub mod schema;
pub mod source;
pub mod translator;
pub mod value;
#[allow(clippy::all, clippy::pedantic)]
#[rustfmt::skip]
pub mod schema_capnp;

pub use compiler::{CompileOptions, CompileOutput, compile_files};
