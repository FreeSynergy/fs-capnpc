//! The [`CompileCommand`] builder.
//!
//! Pattern: Builder — collects options and collaborators, then `run` executes the pipeline.

use crate::compiler::{CompileOptions, CompileOutput, run_compilation};
use crate::error::{CollectingReporter, Error, ErrorReporter};
use crate::pipeline::{DefaultFrontend, Frontend};
use crate::request::{Emitter, RequestEmitter};
use crate::source::{DiskLoader, ModuleLoader, SourceLoader};
use std::path::{Path, PathBuf};

/// Describes one compilation, mirroring `capnpc::CompilerCommand`.
pub struct CompileCommand {
    options: CompileOptions,
    source: Box<dyn SourceLoader>,
    frontend: Box<dyn Frontend>,
    reporter: Box<dyn ErrorReporter>,
    emitter: Box<dyn Emitter>,
}

impl Default for CompileCommand {
    fn default() -> Self {
        CompileCommand {
            options: CompileOptions::default(),
            source: Box::new(DiskLoader),
            frontend: Box::new(DefaultFrontend::standard()),
            reporter: Box::new(CollectingReporter::default()),
            emitter: Box::new(RequestEmitter),
        }
    }
}

impl CompileCommand {
    /// A command reading from disk with the standard stages.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A command from ready-made options.
    #[must_use]
    pub fn from_options(options: CompileOptions) -> Self {
        CompileCommand {
            options,
            ..Self::default()
        }
    }

    /// Add a schema file to compile.
    pub fn file(&mut self, path: impl AsRef<Path>) -> &mut Self {
        self.options.files.push(path.as_ref().to_path_buf());
        self
    }

    /// Strip this prefix from file names to form display names.
    pub fn src_prefix(&mut self, path: impl AsRef<Path>) -> &mut Self {
        self.options.src_prefixes.push(path.as_ref().to_path_buf());
        self
    }

    /// Search this directory for absolute imports.
    pub fn import_path(&mut self, path: impl AsRef<Path>) -> &mut Self {
        self.options.import_paths.push(PathBuf::from(path.as_ref()));
        self
    }

    /// Do not fall back to the embedded standard schema files.
    pub fn no_standard_import(&mut self) -> &mut Self {
        self.options.no_standard_import = true;
        self
    }

    /// Replace where source bytes come from (e.g. an in-memory file set).
    pub fn source_loader(&mut self, source: Box<dyn SourceLoader>) -> &mut Self {
        self.source = source;
        self
    }

    /// Replace the lex/parse front end.
    pub fn frontend(&mut self, frontend: Box<dyn Frontend>) -> &mut Self {
        self.frontend = frontend;
        self
    }

    /// Replace the diagnostics sink.
    pub fn reporter(&mut self, reporter: Box<dyn ErrorReporter>) -> &mut Self {
        self.reporter = reporter;
        self
    }

    /// Replace the final emission stage used by [`CompileCommand::run_bytes`].
    pub fn emitter(&mut self, emitter: Box<dyn Emitter>) -> &mut Self {
        self.emitter = emitter;
        self
    }

    /// Run the pipeline and return the compiled nodes.
    ///
    /// # Errors
    /// Returns every diagnostic when the schemas are invalid.
    pub fn run(self) -> Result<CompileOutput, Error> {
        let loader = ModuleLoader::with(self.source, self.frontend);
        run_compilation(&self.options, loader, self.reporter)
    }

    /// Run the pipeline and emit the result with the configured emitter.
    ///
    /// # Errors
    /// Returns every diagnostic when the schemas are invalid.
    pub fn run_bytes(mut self) -> Result<Vec<u8>, Error> {
        let emitter = std::mem::replace(&mut self.emitter, Box::new(RequestEmitter));
        let out = self.run()?;
        Ok(emitter.emit(&out))
    }
}
