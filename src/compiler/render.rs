//! Rendering of collected diagnostics into a deduplicated, human-readable error.
//!
//! Pattern: Adapter — turns compiler diagnostics into the public `Error` type.

use crate::compiler::core::Compiler;
use crate::error::Error;
use std::collections::HashSet;

pub(super) fn render_errors(c: &Compiler) -> Error {
    let mut rendered = vec![];
    let mut seen = HashSet::new();
    for d in c.reporter.diagnostics() {
        let key = (d.file.clone(), d.span, d.message.clone());
        if !seen.insert(key) {
            continue;
        }
        let src = c
            .loader
            .modules
            .iter()
            .find(|m| m.display_name == d.file)
            .map(|m| m.source.as_slice());
        rendered.push(d.render(src));
    }
    Error {
        diagnostics: c.reporter.diagnostics().to_vec(),
        rendered,
    }
}
