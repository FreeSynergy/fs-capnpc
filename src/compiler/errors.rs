//! Diagnostic reporting and import-resolution recording on the compiler.
//!
//! Pattern: Facade — small reporting API over the diagnostics list and module records.

use crate::compiler::core::Compiler;
use crate::error::{Diagnostic, Span};
use crate::schema::Id;

impl Compiler {
    /// Record an error at `span` in `module`.
    pub fn add_error(&mut self, module: usize, span: Span, message: impl Into<String>) {
        let file = self
            .loader
            .modules
            .get(module)
            .map(|m| m.display_name.clone())
            .unwrap_or_default();
        self.reporter.report(Diagnostic {
            file,
            span,
            message: message.into(),
        });
    }

    /// True when any diagnostic has been recorded.
    pub fn had_errors(&self) -> bool {
        self.reporter.had_errors()
    }

    /// Record that the byte range of a type expression resolved to `type_id`.
    pub fn report_resolution(&mut self, module: usize, start: u32, end: u32, type_id: Id) {
        if let Some(m) = self.loader.modules.get_mut(module) {
            m.resolutions.push(crate::source::Resolution {
                start_byte: start,
                end_byte: end,
                type_id,
            });
        }
    }
}
