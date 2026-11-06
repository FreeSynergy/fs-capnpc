//! Tests for the command builder with an in-memory source.
//!
//! Pattern: Strategy — the pipeline runs unchanged over a [`MemoryLoader`].

use super::CompileCommand;
use crate::source::MemoryLoader;

#[test]
fn compiles_from_memory() {
    let mut mem = MemoryLoader::new();
    mem.add(
        "/w/a.capnp",
        "@0xdbb9ad1f14bf0b36;\nstruct S { x @0 :UInt32; }\n",
    );
    let mut cmd = CompileCommand::new();
    cmd.source_loader(Box::new(mem))
        .src_prefix("/w")
        .file("/w/a.capnp");
    let bytes = cmd.run_bytes().expect("compiles");
    assert!(!bytes.is_empty());
}

#[test]
fn reports_errors() {
    let mut mem = MemoryLoader::new();
    mem.add(
        "/w/a.capnp",
        "@0xdbb9ad1f14bf0b36;\nstruct S { x @0 :Nope; }\n",
    );
    let mut cmd = CompileCommand::new();
    cmd.source_loader(Box::new(mem))
        .src_prefix("/w")
        .file("/w/a.capnp");
    let err = cmd.run().err().expect("fails");
    assert!(err.to_string().contains("Nope"), "{err}");
}
