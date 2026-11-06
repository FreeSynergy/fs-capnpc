//! Dev tool: regenerate `src/schema_capnp.rs` from the capnp 1.4.0 `schema.capnp`
//! (the copy shipped with capnpc-embedded is the reference). Run:
//! `cargo run --example gen_schema -- <path-to-schema.capnp> <out-dir>`
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let schema = &args[1];
    let out = &args[2];
    let dir = std::path::Path::new(schema).parent().unwrap();
    let req = capnpc_embedded::CompileCommand::new()
        .src_prefix(dir)
        .file(schema)
        .compile()
        .unwrap();
    capnpc::codegen::CodeGenerationCommand::new()
        .output_directory(out)
        .run(&req[..])
        .unwrap();
}
