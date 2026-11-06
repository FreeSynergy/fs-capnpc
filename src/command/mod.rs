//! Public entry point of the compiler.
//!
//! Pattern: Facade + Builder — [`CompileCommand`] hides the pipeline (load, lex, parse, resolve,
//! translate, emit) behind a fluent builder.

mod builder;
mod run;
#[cfg(test)]
mod tests;

pub use builder::CompileCommand;
pub use run::compile_files;
