//! Loaded-module records.
//!
//! Pattern: Newtype-style plain records — data carried between the loader and the compiler.

use super::paths::Dir;
use crate::ast::Declaration;
use std::rc::Rc;

/// A location where an identifier was resolved to a type id (for `FileSourceInfo`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resolution {
    /// Start byte of the type expression.
    pub start_byte: u32,
    /// End byte of the type expression.
    pub end_byte: u32,
    /// Id of the type it resolved to.
    pub type_id: u64,
}

/// One parsed schema file.
pub struct Module {
    /// Name shown in node display names (path below the source prefix).
    pub display_name: String,
    /// Directory the file was found in.
    pub dir: Dir,
    /// Path components below `dir`.
    pub path: Vec<String>,
    /// The raw source bytes.
    pub source: Vec<u8>,
    /// The parsed top-level file declaration.
    pub root: Rc<Declaration>,
    /// Resolutions recorded while compiling.
    pub resolutions: Vec<Resolution>,
}
