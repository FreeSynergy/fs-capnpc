//! The [`ErrorReporter`] interface and its collecting implementation.
//!
//! Pattern: Strategy + Observer — stages report diagnostics to an injected sink (collect, print,
//! forward to an editor) without knowing which.

use super::diagnostic::Diagnostic;

/// Receives diagnostics from every compiler stage.
pub trait ErrorReporter {
    /// Record one diagnostic.
    fn report(&mut self, diagnostic: Diagnostic);

    /// Everything reported so far, in order.
    fn diagnostics(&self) -> &[Diagnostic];

    /// Record several diagnostics.
    fn report_all(&mut self, diagnostics: Vec<Diagnostic>) {
        for d in diagnostics {
            self.report(d);
        }
    }

    /// True when at least one diagnostic was reported.
    fn had_errors(&self) -> bool {
        !self.diagnostics().is_empty()
    }
}

/// Keeps all diagnostics in a vector.
#[derive(Clone, Debug, Default)]
pub struct CollectingReporter {
    items: Vec<Diagnostic>,
}

impl ErrorReporter for CollectingReporter {
    fn report(&mut self, diagnostic: Diagnostic) {
        self.items.push(diagnostic);
    }

    fn diagnostics(&self) -> &[Diagnostic] {
        &self.items
    }
}
