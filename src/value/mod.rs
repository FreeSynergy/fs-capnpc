//! Constant / default value evaluation (the `ValueTranslator` part of `node-translator.c++`).
//!
//! Expressions are evaluated against an expected [`Type`](crate::schema::Type) and encoded
//! straight into the pointer trees stored in schema `Value`s.
//!
//! Pattern: Facade — this module only re-exports the `compile_value` entry point.

mod constants;
mod convert;
mod dynval;
mod expr;
mod lists;
mod structs;
mod subst;
mod vt;

pub use vt::compile_value;
