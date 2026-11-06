//! Free-function convenience over [`CompileCommand`].
//!
//! Pattern: Facade — the shortest path from options to compiled output.

use super::builder::CompileCommand;
use crate::compiler::{CompileOptions, CompileOutput};
use crate::error::Error;

/// Compile the files in `opts` from disk with the standard pipeline.
///
/// # Errors
/// Returns all diagnostics rendered with line/column.
pub fn compile_files(opts: &CompileOptions) -> Result<CompileOutput, Error> {
    CompileCommand::from_options(opts.clone()).run()
}
