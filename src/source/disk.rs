//! File-system [`SourceLoader`].
//!
//! Pattern: Strategy — the default implementation reading from disk and the embedded standard files.

use super::paths::{Dir, join_path};
use super::provider::SourceLoader;
use super::std_files::std_file;

/// Reads schema files from the file system; `Dir::Std` is served from the embedded copies.
#[derive(Clone, Copy, Debug, Default)]
pub struct DiskLoader;

impl SourceLoader for DiskLoader {
    fn read(&self, dir: &Dir, path: &[String]) -> Option<Vec<u8>> {
        match dir {
            Dir::Disk(root) => {
                let p = join_path(root, path);
                if p.is_file() {
                    std::fs::read(&p).ok()
                } else {
                    None
                }
            }
            Dir::Std => std_file(path),
        }
    }

    fn identity(&self, dir: &Dir, path: &[String]) -> String {
        match dir {
            Dir::Disk(root) => {
                let p = join_path(root, path);
                std::fs::canonicalize(&p)
                    .unwrap_or(p)
                    .to_string_lossy()
                    .into_owned()
            }
            Dir::Std => format!("std:{}", path.join("/")),
        }
    }
}
