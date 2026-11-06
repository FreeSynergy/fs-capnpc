//! The [`SourceLoader`] interface.
//!
//! Pattern: Strategy — the compiler reads schema bytes through this trait only, so the file system
//! and in-memory sources are interchangeable (tests, editors, embedding).

use super::paths::Dir;

/// Supplies the bytes of schema files.
pub trait SourceLoader {
    /// Bytes of the file at `path` (components) below `dir`, or `None` when it does not exist.
    fn read(&self, dir: &Dir, path: &[String]) -> Option<Vec<u8>>;

    /// A stable identity for the file, used to cache modules (two names for one file share it).
    fn identity(&self, dir: &Dir, path: &[String]) -> String;
}
