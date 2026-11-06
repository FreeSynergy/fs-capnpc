//! Module cache and import resolution on top of a [`SourceLoader`] and a [`Frontend`].
//!
//! Pattern: Facade — hides lookup rules (relative, absolute, import paths, std files), caching and
//! parsing behind a handful of calls; both collaborators are injected as trait objects.

use super::module::Module;
use super::paths::{Dir, abs_components, comps_to_path, eval_components};
use super::provider::SourceLoader;
use crate::error::Diagnostic;
use crate::pipeline::{DefaultFrontend, Frontend};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Loads, parses and caches schema modules.
pub struct ModuleLoader {
    /// Directories searched for absolute imports.
    pub import_paths: Vec<PathBuf>,
    /// Prefixes stripped from file names to form display names.
    pub src_prefixes: Vec<PathBuf>,
    /// Do not fall back to the embedded standard files.
    pub no_standard_import: bool,
    /// All modules loaded so far (index = module id).
    pub modules: Vec<Module>,
    by_key: HashMap<String, usize>,
    source: Box<dyn SourceLoader>,
    frontend: Box<dyn Frontend>,
}

impl Default for ModuleLoader {
    fn default() -> Self {
        Self::with(
            Box::new(super::DiskLoader),
            Box::new(DefaultFrontend::standard()),
        )
    }
}

impl ModuleLoader {
    /// A loader reading from disk with the default front end.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A loader with explicit collaborators (Strategy injection).
    #[must_use]
    pub fn with(source: Box<dyn SourceLoader>, frontend: Box<dyn Frontend>) -> Self {
        ModuleLoader {
            import_paths: vec![],
            src_prefixes: vec![],
            no_standard_import: false,
            modules: vec![],
            by_key: HashMap::new(),
            source,
            frontend,
        }
    }

    /// Work out the directory and relative path for a file named on the command line.
    #[must_use]
    pub fn interpret_source_file(&self, file: &Path) -> (Dir, Vec<String>) {
        let path = abs_components(file);
        let prefixes: Vec<Vec<String>> = self
            .src_prefixes
            .iter()
            .map(|p| abs_components(p))
            .collect();
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

    /// Load (or fetch from the cache) a module. Parse errors are appended to `errors`.
    pub fn load_module(
        &mut self,
        dir: &Dir,
        path: &[String],
        errors: &mut Vec<Diagnostic>,
    ) -> Option<usize> {
        if path.is_empty() {
            return None;
        }
        let key = self.source.identity(dir, path);
        if let Some(&i) = self.by_key.get(&key) {
            return Some(i);
        }
        let source = self.source.read(dir, path)?;
        let display_name = path.join("/");
        let (root, diags) = self.frontend.parse_source(&source, &display_name);
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

    fn absolute_dirs(&self) -> Vec<Dir> {
        let mut dirs: Vec<Dir> = self
            .import_paths
            .iter()
            .map(|p| Dir::Disk(p.clone()))
            .collect();
        if !self.no_standard_import {
            dirs.push(Dir::Std);
        }
        dirs
    }

    /// Import `import_path` from module `from`.
    pub fn import_relative(
        &mut self,
        from: usize,
        import_path: &str,
        errors: &mut Vec<Diagnostic>,
    ) -> Option<usize> {
        if let Some(abs) = import_path.strip_prefix('/') {
            let comps = eval_components(&[], abs)?;
            self.absolute_dirs()
                .into_iter()
                .find_map(|d| self.load_module(&d, &comps, errors))
        } else {
            let (dir, base) = {
                let m = &self.modules[from];
                (m.dir.clone(), m.path[..m.path.len() - 1].to_vec())
            };
            let comps = eval_components(&base, import_path)?;
            self.load_module(&dir, &comps, errors)
        }
    }

    /// Bytes of a file named by an `embed` expression in module `from`.
    #[must_use]
    pub fn embed_relative(&self, from: usize, embed_path: &str) -> Option<Vec<u8>> {
        if let Some(abs) = embed_path.strip_prefix('/') {
            let comps = eval_components(&[], abs)?;
            self.absolute_dirs()
                .iter()
                .find_map(|d| self.source.read(d, &comps))
        } else {
            let m = &self.modules[from];
            let base = m.path[..m.path.len() - 1].to_vec();
            let comps = eval_components(&base, embed_path)?;
            self.source.read(&m.dir, &comps)
        }
    }
}
