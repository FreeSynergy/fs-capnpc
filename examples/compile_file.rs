//! Compile schema files from disk and generate Rust with `capnpc`, like a `build.rs` would.
//!
//! Pattern: Facade — the three steps (compile, emit request, generate code) behind one builder.
//!
//! Run with `cargo run --example compile_file -- path/to/schema.capnp out-dir`.

use capnp_compiler::CompileCommand;

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(schema), Some(out)) = (args.next(), args.next()) else {
        eprintln!("usage: compile_file <schema.capnp> <out-dir>");
        return;
    };
    let mut cmd = CompileCommand::new();
    cmd.file(&schema);
    let request = cmd.run_bytes().unwrap_or_else(|e| panic!("{e}"));
    std::fs::create_dir_all(&out).expect("create output directory");
    capnpc::codegen::CodeGenerationCommand::new()
        .output_directory(&out)
        .run(&request[..])
        .expect("code generation");
    println!("generated into {out}");
}
