//! Compile a schema held in memory and print how large the request is.
//!
//! Pattern: Facade + Strategy — `CompileCommand` with a `MemoryLoader` instead of the file system.
//!
//! Run with `cargo run --example in_memory`.

use capnp_compiler::CompileCommand;
use capnp_compiler::source::MemoryLoader;

fn main() {
    let mut files = MemoryLoader::new();
    files.add(
        "/work/point.capnp",
        "@0xdbb9ad1f14bf1001;\nstruct Point { x @0 :Int32; y @1 :Int32; }\n",
    );
    let mut cmd = CompileCommand::new();
    cmd.source_loader(Box::new(files))
        .src_prefix("/work")
        .file("/work/point.capnp");
    match cmd.run_bytes() {
        Ok(bytes) => println!("CodeGeneratorRequest: {} bytes", bytes.len()),
        Err(e) => eprintln!("{e}"),
    }
}
