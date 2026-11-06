//! Public option and output types of [`compile_files`](crate::compiler::compile_files).
//!
//! Pattern: Builder — plain option struct filled by callers, plain output struct consumed by writers.

use crate::schema::{Id, Node, SourceInfo};
use std::path::PathBuf;
use std::rc::Rc;

/// Options for [`compile_files`].
#[derive(Clone, Debug, Default)]
pub struct CompileOptions {
    /// Schema files to compile.
    pub files: Vec<PathBuf>,
    /// Prefixes stripped from file names to form display names.
    pub src_prefixes: Vec<PathBuf>,
    /// Directories searched for imports.
    pub import_paths: Vec<PathBuf>,
    /// Disable the implicit standard import paths.
    pub no_standard_import: bool,
}

/// Everything needed to build a `CodeGeneratorRequest`.
pub struct CompileOutput {
    /// All nodes in official output order.
    pub nodes: Vec<Rc<Node>>,
    /// Source info in official output order.
    pub source_info: Vec<SourceInfo>,
    /// Requested files with their imports and resolutions.
    pub requested_files: Vec<RequestedFileOut>,
}

/// A requested file in the final output.
pub struct RequestedFileOut {
    /// File node id.
    pub id: Id,
    /// Display file name.
    pub filename: String,
    /// Imports as (id, name).
    pub imports: Vec<(Id, String)>,
    /// Resolved type-expression ranges for source info.
    pub resolutions: Vec<crate::source::Resolution>,
}
