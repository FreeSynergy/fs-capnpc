//! The compilation driver: load, compile and collect everything for a request.
//!
//! Pattern: Template method — fixed sequence (load -> eager compile -> collect) over injected
//! collaborators (module loader, error reporter).

use crate::compiler::core::Compiler;
use crate::compiler::kinds::DEFAULT_EAGERNESS;
use crate::compiler::node_data::RequestedFile;
use crate::compiler::options::*;
use crate::compiler::render::render_errors;
use crate::error::{Diagnostic, Error, ErrorReporter, Span};
use crate::schema::Id;
use crate::source::ModuleLoader;

/// Compile `opts.files` using the given collaborators.
///
/// # Errors
/// On errors returns all diagnostics rendered with line/column.
pub fn run_compilation(
    opts: &CompileOptions,
    mut loader: ModuleLoader,
    reporter: Box<dyn ErrorReporter>,
) -> Result<CompileOutput, Error> {
    loader.import_paths.clone_from(&opts.import_paths);
    loader.src_prefixes.clone_from(&opts.src_prefixes);
    loader.no_standard_import = opts.no_standard_import;
    let mut c = Compiler::new(loader);
    c.reporter = reporter;

    for f in &opts.files {
        let (dir, path) = c.loader.interpret_source_file(f);
        let mut errs = vec![];
        let m = c.loader.load_module(&dir, &path, &mut errs);
        c.reporter.report_all(errs);
        let Some(m) = m else {
            c.reporter.report(Diagnostic {
                file: f.display().to_string(),
                span: Span::default(),
                message: "no such file".into(),
            });
            continue;
        };
        let root = c.add_module(m);
        c.eagerly_compile(root, DEFAULT_EAGERNESS);
        let id = c.nodes[root].id;
        let filename = c.loader.modules[m].display_name.clone();
        c.requested.push(RequestedFile {
            id,
            filename,
            imports: vec![],
            module: m,
        });
    }

    if c.had_errors() {
        return Err(render_errors(&c));
    }

    let nodes = c.sim.all_loaded();
    let mut files = vec![];
    let reqs: Vec<(Id, String, usize)> = c
        .requested
        .iter()
        .map(|r| (r.id, r.filename.clone(), r.module))
        .collect();
    for (id, filename, module) in reqs {
        let imports = c.file_import_table(module);
        let resolutions = c.loader.modules[module].resolutions.clone();
        files.push(RequestedFileOut {
            id,
            filename,
            imports,
            resolutions,
        });
    }
    if c.had_errors() {
        return Err(render_errors(&c));
    }
    let source_info =
        crate::hashorder::order_source_info(std::mem::take(&mut c.source_info_inserted));
    Ok(CompileOutput {
        nodes,
        source_info,
        requested_files: files,
    })
}
