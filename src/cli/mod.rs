//! Command line interface: `capnp-compiler compile -o <dir> files...`.
//!
//! Pattern: Command — parsed arguments become a [`Command`] value that `run` executes.

mod args;
mod run;
#[cfg(test)]
mod tests;

pub use args::{Command, CompileArgs, parse_args};
pub use run::run;
