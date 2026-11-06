//! Invalid schemas must be rejected by both compilers.
//!
//! Pattern: Table-driven test — each row is a schema that the official compiler refuses.

mod common;

use capnp_compiler::CompileCommand;
use capnp_compiler::source::MemoryLoader;

const HEADER: &str = "@0xdbb9ad1f14bf0d01;\n";

const INVALID: &[(&str, &str)] = &[
    ("unknown type", "struct S { a @0 :Nope; }"),
    ("skipped ordinal", "struct S { a @1 :Int32; }"),
    ("duplicate ordinal", "struct S { a @0 :Int32; b @0 :Int32; }"),
    ("duplicate name", "struct S { a @0 :Int32; a @1 :Int32; }"),
    ("bad default", "struct S { a @0 :Int8 = 300; }"),
    ("enum default mismatch", "enum E { a @0; }\nstruct S { x @0 :E = 3; }"),
    ("union with one member", "struct S { union { a @0 :Int32; } }"),
    ("missing semicolon", "struct S { a @0 :Int32 }"),
    ("generic arity", "struct G(T) { a @0 :T; }\nstruct S { g @0 :G(Int32, Int32); }"),
    ("annotation target", "annotation a(struct) :Void;\nenum E $a { x @0; }"),
    ("const type mismatch", "const c :Text = 5;"),
    ("lowercase struct", "struct s { a @0 :Int32; }"),
    ("duplicate id", "struct A @0x8000000000000001 {}\nstruct B @0x8000000000000001 {}"),
    ("interface param", "interface I { m @0 (a :Int32, a :Int32) -> (); }"),
    ("list of nothing", "struct S { a @0 :List; }"),
];

#[test]
fn both_compilers_reject_invalid_schemas() {
    let dir = std::env::temp_dir().join(format!("capnp-compiler-err-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut mismatches = vec![];
    for (i, (name, body)) in INVALID.iter().enumerate() {
        let text = format!("{HEADER}{body}\n");
        let path = dir.join(format!("e{i}.capnp"));
        std::fs::write(&path, &text).unwrap();
        let oracle = std::panic::catch_unwind(|| {
            common::oracle_bytes(&[path.to_str().unwrap()], dir.to_str().unwrap(), &[])
        });
        let mut mem = MemoryLoader::new();
        mem.add(&format!("/w/e{i}.capnp"), text.clone());
        let mut cmd = CompileCommand::new();
        cmd.source_loader(Box::new(mem)).src_prefix("/w").file(format!("/w/e{i}.capnp"));
        let mine = cmd.run();
        if oracle.is_ok() == mine.is_err() {
            mismatches.push(format!("{name}: oracle ok={} mine err={}", oracle.is_ok(), mine.is_err()));
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}
