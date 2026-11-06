//! Source file discovery and caching (counterpart of `module-loader.c++`).
//!
//! Files are looked up relative to the importing file (`"foo.capnp"`), or, for absolute imports
//! (`"/capnp/c++.capnp"`), in the import path directories and finally in the embedded copies of
//! the standard schemas.

use crate::ast::Declaration;
use crate::error::Diagnostic;
use crate::lexer;
use crate::parser;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Embedded standard schema files (`/capnp/*.capnp`).
pub const STD_FILES: &[(&str, &str)] = &[
    ("capnp/c++.capnp", include_str!("../std/capnp/c++.capnp")),
    ("capnp/schema.capnp", include_str!("../std/capnp/schema.capnp")),
    ("capnp/stream.capnp", include_str!("../std/capnp/stream.capnp")),
    ("capnp/persistent.capnp", include_str!("../std/capnp/persistent.capnp")),
    ("capnp/rpc.capnp", include_str!("../std/capnp/rpc.capnp")),
    ("capnp/rpc-twoparty.capnp", include_str!("../std/capnp/rpc-twoparty.capnp")),
    ("capnp/rust.capnp", include_str!("../std/capnp/rust.capnp")),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dir {
    Disk(PathBuf),
    Std,
}

/// A location where an identifier was resolved to a type id (for `FileSourceInfo`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resolution {
    pub start_byte: u32,
    pub end_byte: u32,
    pub type_id: u64,
}

pub struct Module {
    pub display_name: String,
    pub dir: Dir,
    pub path: Vec<String>,
    pub source: Vec<u8>,
    pub root: Rc<Declaration>,
    pub resolutions: Vec<Resolution>,
}

#[derive(Default)]
pub struct SourceLoader {
    pub import_paths: Vec<PathBuf>,
    pub src_prefixes: Vec<PathBuf>,
    pub no_standard_import: bool,
    pub modules: Vec<Module>,
    by_key: HashMap<String, usize>,
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

fn abs_components(p: &Path) -> Vec<String> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
    let joined = if p.is_absolute() { p.to_path_buf() } else { cwd.join(p) };
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

fn comps_to_path(c: &[String]) -> PathBuf {
    let mut p = PathBuf::from("/");
    for x in c {
        p.push(x);
    }
    p
}

impl SourceLoader {
    pub fn new() -> Self {
        Self::default()
    }

    /// Work out the directory and relative path for a file named on the command line.
    pub fn interpret_source_file(&self, file: &Path) -> (Dir, Vec<String>) {
        let path = abs_components(file);
        let prefixes: Vec<Vec<String>> = self.src_prefixes.iter().map(|p| abs_components(p)).collect();
        for i in (1..path.len()).rev() {
            let prefix = &path[..i];
            if prefixes.iter().any(|p| p.as_slice() == prefix) {
                return (Dir::Disk(comps_to_path(prefix)), path[i..].to_vec());
            }
        }
        let cwd = abs_components(Path::new("."));
        if path.len() >= cwd.len() && path[..cwd.len()] == cwd[..] {
            (Dir::Disk(comps_to_path(&cwd)), path[cwd.len()..].to_vec())
        } else {
            (Dir::Disk(PathBuf::from("/")), path)
        }
    }

    fn read(dir: &Dir, path: &[String]) -> Option<Vec<u8>> {
        match dir {
            Dir::Disk(root) => {
                let mut p = root.clone();
                for c in path {
                    p.push(c);
                }
                if p.is_file() { std::fs::read(&p).ok() } else { None }
            }
            Dir::Std => {
                let name = path.join("/");
                STD_FILES.iter().find(|(n, _)| *n == name).map(|(_, c)| c.as_bytes().to_vec())
            }
        }
    }

    fn key(dir: &Dir, path: &[String]) -> String {
        match dir {
            Dir::Disk(root) => {
                let mut p = root.clone();
                for c in path {
                    p.push(c);
                }
                std::fs::canonicalize(&p).unwrap_or(p).to_string_lossy().into_owned()
            }
            Dir::Std => format!("std:{}", path.join("/")),
        }
    }

    /// Load (or fetch from the cache) a module. Parse errors are appended to `errors`.
    pub fn load_module(&mut self, dir: &Dir, path: &[String], errors: &mut Vec<Diagnostic>) -> Option<usize> {
        if path.is_empty() {
            return None;
        }
        let key = Self::key(dir, path);
        if let Some(&i) = self.by_key.get(&key) {
            return Some(i);
        }
        let source = Self::read(dir, path)?;
        let display_name = path.join("/");
        let mut diags = vec![];
        let root = match lexer::lex(&source, &display_name) {
            Ok(stmts) => {
                let (parsed, errs) = parser::parse_file(&stmts, &display_name, true);
                diags.extend(errs);
                parsed.root
            }
            Err(errs) => {
                diags.extend(errs);
                let (parsed, _) = parser::parse_file(&[], &display_name, false);
                parsed.root
            }
        };
        errors.extend(diags);
        let idx = self.modules.len();
        self.modules.push(Module {
            display_name,
            dir: dir.clone(),
            path: path.to_vec(),
            source,
            root,
            resolutions: vec![],
        });
        self.by_key.insert(key, idx);
        Some(idx)
    }

    /// Import `import_path` from module `from`.
    pub fn import_relative(&mut self, from: usize, import_path: &str, errors: &mut Vec<Diagnostic>) -> Option<usize> {
        if let Some(abs) = import_path.strip_prefix('/') {
            let comps = eval_components(&[], abs)?;
            let mut dirs: Vec<Dir> = self.import_paths.iter().map(|p| Dir::Disk(p.clone())).collect();
            if !self.no_standard_import {
                dirs.push(Dir::Std);
            }
            for d in dirs {
                if let Some(m) = self.load_module(&d, &comps, errors) {
                    return Some(m);
                }
            }
            None
        } else {
            let (dir, base) = {
                let m = &self.modules[from];
                (m.dir.clone(), m.path[..m.path.len() - 1].to_vec())
            };
            let comps = eval_components(&base, import_path)?;
            self.load_module(&dir, &comps, errors)
        }
    }

    pub fn embed_relative(&self, from: usize, embed_path: &str) -> Option<Vec<u8>> {
        if let Some(abs) = embed_path.strip_prefix('/') {
            let comps = eval_components(&[], abs)?;
            for p in &self.import_paths {
                if let Some(d) = Self::read(&Dir::Disk(p.clone()), &comps) {
                    return Some(d);
                }
            }
            if !self.no_standard_import {
                return Self::read(&Dir::Std, &comps);
            }
            None
        } else {
            let m = &self.modules[from];
            let base = m.path[..m.path.len() - 1].to_vec();
            let comps = eval_components(&base, embed_path)?;
            Self::read(&m.dir, &comps)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval() {
        let base = vec!["a".to_string(), "b".to_string()];
        assert_eq!(eval_components(&base, "../c.capnp"), Some(vec!["a".into(), "c.capnp".into()]));
        assert_eq!(eval_components(&base, "../../../x"), None);
        assert_eq!(eval_components(&[], "capnp/c++.capnp"), Some(vec!["capnp".into(), "c++.capnp".into()]));
    }

    #[test]
    fn std_import() {
        let mut l = SourceLoader::new();
        let mut errs = vec![];
        let m = l.load_module(&Dir::Std, &["capnp".into(), "c++.capnp".into()], &mut errs).unwrap();
        assert!(errs.is_empty(), "{errs:?}");
        let r = l.import_relative(m, "/capnp/stream.capnp", &mut errs);
        assert!(r.is_some());
    }
}
