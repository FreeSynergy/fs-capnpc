//! Dev tool: regenerate `src/schema_capnp.rs`.
//!
//! The reference compiler embedded in `capnpc-embedded` (capnp 1.4.0 as WASM) emits
//! `Node.SourceInfo.Member` structs with one data word and two pointers, which is larger than the
//! `schema.capnp` shipped next to it declares. To reproduce the encoding byte for byte this tool
//! patches the schema (extra, unused `Member` fields) before running the code generator; then
//! `scripts/patch_schema_capnp.py` adds one helper method.
//!
//! Usage: `cargo run --example gen_schema -- <out-dir>` and then
//! `python3 scripts/patch_schema_capnp.py <out-dir>/schema_capnp.rs`.
fn main() {
    let out = std::env::args().nth(1).expect("output directory");
    let src = std::fs::read_to_string("std/capnp/schema.capnp").expect("std/capnp/schema.capnp");
    let patched = src.replace(
        "      docComment @0 :Text;\n      # Doc comment on the member.\n    }",
        "      docComment @0 :Text;\n      # Doc comment on the member.\n      startByte @1 :UInt32;\n      endByte @2 :UInt32;\n      reserved @3 :Text;\n    }",
    );
    assert_ne!(src, patched, "Member struct not found");
    let dir = std::env::temp_dir().join("capnp-compiler-gen-schema");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy("std/capnp/c++.capnp", dir.join("c++.capnp")).unwrap();
    let file = dir.join("schema.capnp");
    std::fs::write(&file, patched).unwrap();
    let req = capnpc_embedded::CompileCommand::new()
        .src_prefix(&dir)
        .file(&file)
        .compile()
        .unwrap();
    capnpc::codegen::CodeGenerationCommand::new()
        .output_directory(&out)
        .run(&req[..])
        .unwrap();
}
