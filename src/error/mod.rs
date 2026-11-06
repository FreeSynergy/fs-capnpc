//! Diagnostics (counterpart of `error-reporter.c++`).
//!
//! Pattern: Strategy — [`ErrorReporter`] is the sink interface; spans and diagnostics are values.

mod diagnostic;
mod failure;
mod reporter;
mod span;

pub use diagnostic::{Diagnostic, line_col};
pub use failure::Error;
pub use reporter::{CollectingReporter, ErrorReporter};
pub use span::Span;
