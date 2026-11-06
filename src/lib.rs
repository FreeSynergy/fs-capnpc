//! `capnp-compiler`: a Cap'n Proto schema compiler in pure Rust.

pub mod ast;
pub mod compiler;
pub mod generics;
pub mod hashorder;
pub mod translator;
pub mod value;
pub mod error;
pub mod ids;
pub mod layout;
pub mod lexer;
pub mod schema;
pub mod source;
pub mod parser;
pub mod request;
#[allow(clippy::all, clippy::pedantic)]
#[rustfmt::skip]
pub mod schema_capnp;

pub use compiler::{compile_files, CompileOptions, CompileOutput};
