//! `capnp-compiler`: a Cap'n Proto schema compiler in pure Rust.
//!
//! Pattern: Facade — [`CompileCommand`] is the entry point; the pipeline stages behind it
//! (source loading, lexing, parsing, resolution, translation, emission) are traits, see `DESIGN.md`.
#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![cfg_attr(
    test,
    allow(
        clippy::assert_is_empty,
        reason = "emptiness asserts read clearer than slice comparisons"
    )
)]
#![allow(
    clippy::redundant_closure,
    clippy::redundant_closure_for_method_calls,
    reason = "fn-item paths to elided-lifetime parser methods fail higher-ranked inference"
)]
#![allow(
    clippy::match_same_arms,
    clippy::match_wildcard_for_single_variants,
    clippy::unused_self,
    reason = "explicit arms and uniform receivers keep stage code parallel to the reference"
)]
#![allow(
    clippy::cast_precision_loss,
    reason = "numeric literal conversion mirrors the reference compiler"
)]
#![allow(
    clippy::module_name_repetitions,
    reason = "re-exported names read better with their concept prefix"
)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    reason = "internal stages share one error type"
)]
#![allow(
    clippy::must_use_candidate,
    clippy::return_self_not_must_use,
    reason = "builders and getters are used for effect"
)]
#![allow(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    reason = "ported compiler routines mirror the C++ reference"
)]
#![allow(
    clippy::many_single_char_names,
    clippy::unreadable_literal,
    clippy::similar_names,
    clippy::struct_excessive_bools,
    clippy::wildcard_imports,
    clippy::enum_glob_use,
    clippy::items_after_statements,
    clippy::doc_markdown,
    reason = "hash constants, grammar flags and glob-imported kinds follow the reference implementation"
)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    reason = "Cap'n Proto layout arithmetic uses fixed-width fields"
)]

pub mod ast;
pub mod cli;
pub mod command;
pub mod compiler;
pub mod error;
pub mod generics;
mod hashmath;
pub mod hashorder;
pub mod ids;
pub mod layout;
pub mod lexer;
pub mod parser;
pub mod pipeline;
pub mod request;
pub mod schema;
pub mod source;
pub mod translator;
pub mod value;
#[allow(clippy::all, clippy::pedantic, missing_docs)]
#[rustfmt::skip]
pub mod schema_capnp;

pub use command::{CompileCommand, compile_files};
pub use compiler::{CompileOptions, CompileOutput};
