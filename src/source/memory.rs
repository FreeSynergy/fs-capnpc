//! In-memory [`SourceLoader`].
//!
//! Pattern: Strategy — an alternative backend holding virtual files, for tests and embedding.

use super::paths::{Dir, join_path};
use super::provider::SourceLoader;
use super::std_files::std_file;
use std::collections::HashMap;

/// Serves schema files from a map of virtual absolute paths (`/dir/file.capnp`).
#[derive(Clone, Debug, Default)]
pub struct MemoryLoader {
    files: HashMap<String, Vec<u8>>,
}

impl MemoryLoader {
    /// An empty file set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add (or replace) a virtual file, e.g. `add("/src/a.capnp", "...")`.
    pub fn add(&mut self, path: &str, content: impl Into<Vec<u8>>) -> &mut Self {
        self.files.insert(path.to_string(), content.into());
        self
    }
}

impl SourceLoader for MemoryLoader {
    fn read(&self, dir: &Dir, path: &[String]) -> Option<Vec<u8>> {
        match dir {
            Dir::Disk(root) => self.files.get(join_path(root, path).to_str()?).cloned(),
            Dir::Std => std_file(path),
        }
    }

    fn identity(&self, dir: &Dir, path: &[String]) -> String {
        match dir {
            Dir::Disk(root) => join_path(root, path).to_string_lossy().into_owned(),
            Dir::Std => format!("std:{}", path.join("/")),
        }
    }
}
