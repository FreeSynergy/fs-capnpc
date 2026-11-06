//! CLI tests: argument parsing and an end-to-end run.
//!
//! Pattern: Command — commands are plain values, so parsing is tested without side effects.

use super::{Command, parse_args, run};

fn argv(s: &str) -> Vec<String> {
    s.split_whitespace().map(String::from).collect()
}

#[test]
fn parses_compile() {
    let Ok(Command::Compile(a)) = parse_args(argv(
        "compile -o out -I inc --src-prefix src a.capnp b.capnp",
    )) else {
        panic!("expected compile");
    };
    assert_eq!(a.files.len(), 2);
    assert_eq!(a.import_paths.len(), 1);
    assert_eq!(a.output_dir.as_deref(), Some(std::path::Path::new("out")));
}

#[test]
fn rejects_bad_input() {
    assert!(parse_args(argv("compile -o out")).is_err());
    assert!(parse_args(argv("compile --bogus a.capnp")).is_err());
    assert!(parse_args(argv("frobnicate")).is_err());
    assert!(parse_args(argv("compile a.capnp")).is_err());
    assert_eq!(parse_args(argv("--help")), Ok(Command::Help));
}

#[test]
fn end_to_end_generates_rust() {
    let dir = std::env::temp_dir().join(format!("capnp-compiler-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let schema = dir.join("hello.capnp");
    std::fs::write(
        &schema,
        "@0xdbb9ad1f14bf0f01;\nstruct Hello { name @0 :Text; }\n",
    )
    .unwrap();
    let out = dir.join("out");
    let code = run(vec![
        "compile".into(),
        "-o".into(),
        out.display().to_string(),
        "--src-prefix".into(),
        dir.display().to_string(),
        schema.display().to_string(),
    ]);
    assert_eq!(code, 0);
    let text = std::fs::read_to_string(out.join("hello_capnp.rs")).unwrap();
    assert!(text.contains("pub mod hello"));
    let _ = std::fs::remove_dir_all(&dir);
}
