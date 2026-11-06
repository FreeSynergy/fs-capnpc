//! Path arithmetic for schema lookup: directories, lexical normalisation, absolute components.
//!
//! Pattern: Newtype-style helpers — pure functions over path components, no I/O of their own.

use std::path::{Path, PathBuf};

/// A search location: a directory on disk or the embedded standard files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dir {
    /// A directory in the file system (or virtual root).
    Disk(PathBuf),
    /// The embedded `/capnp/*.capnp` files.
    Std,
}

/// Lexically normalise a path into components (resolves `.` and `..`). Returns `None` when the path
/// escapes the root.
pub fn eval_components(base: &[String], rel: &str) -> Option<Vec<String>> {
    let mut out: Vec<String> = base.to_vec();
    for part in rel.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                out.pop()?;
            }
            p => out.push(p.to_string()),
        }
    }
    Some(out)
}

pub(super) fn abs_components(p: &Path) -> Vec<String> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
    let joined = if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    };
    let mut out: Vec<String> = vec![];
    for c in joined.components() {
        use std::path::Component::*;
        match c {
            Normal(s) => out.push(s.to_string_lossy().into_owned()),
            ParentDir => {
                out.pop();
            }
            _ => {}
        }
    }
    out
}

pub(super) fn comps_to_path(c: &[String]) -> PathBuf {
    let mut p = PathBuf::from("/");
    for x in c {
        p.push(x);
    }
    p
}

/// Join path components below `root`.
pub(super) fn join_path(root: &Path, comps: &[String]) -> PathBuf {
    let mut p = root.to_path_buf();
    for c in comps {
        p.push(c);
    }
    p
}
