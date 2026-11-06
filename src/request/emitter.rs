//! The [`Emitter`] interface: compile output to bytes.
//!
//! Pattern: Strategy — the last pipeline stage is replaceable (request bytes, JSON dump, ...).

use super::assemble::request_bytes;
use crate::compiler::CompileOutput;

/// Turns the compiler output into an artefact.
pub trait Emitter {
    /// Serialise `out`.
    fn emit(&self, out: &CompileOutput) -> Vec<u8>;
}

/// Emits a serialised `CodeGeneratorRequest` (what `capnpc` plugins consume).
#[derive(Clone, Copy, Debug, Default)]
pub struct RequestEmitter;

impl Emitter for RequestEmitter {
    fn emit(&self, out: &CompileOutput) -> Vec<u8> {
        request_bytes(out)
    }
}
