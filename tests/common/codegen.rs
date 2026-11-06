//! Compare the Rust code `capnpc::codegen` generates from two requests.
//!
//! Pattern: Adapter — wraps the capnpc code generator behind a bytes-in, text-out function.

use std::collections::BTreeMap;
use std::path::Path;

/// Run the code generator over serialized request `bytes`; returns file name -> generated text.
pub fn generate(bytes: &[u8], dir: &Path) -> Result<BTreeMap<String, String>, String> {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).expect("create output dir");
    capnpc::codegen::CodeGenerationCommand::new()
        .output_directory(dir)
        .run(bytes)
        .map_err(|e| e.to_string())?;
    let mut files = BTreeMap::new();
    for e in std::fs::read_dir(dir).expect("read output dir").flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        files.insert(name, std::fs::read_to_string(e.path()).unwrap_or_default());
    }
    Ok(files)
}

/// Describe the first difference between the generated outputs, if any.
pub fn diff_generated(oracle: &[u8], mine: &[u8], tag: &str) -> Option<String> {
    let base =
        std::env::temp_dir().join(format!("capnp-compiler-gen-{tag}-{}", std::process::id()));
    let a = generate(oracle, &base.join("oracle"));
    let b = generate(mine, &base.join("mine"));
    let _ = std::fs::remove_dir_all(&base);
    // A schema capnpc cannot generate code for must fail identically for both requests.
    let (a, b) = match (a, b) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(x), Err(y)) if x == y => return None,
        (x, y) => return Some(format!("code generation results differ: {x:?} vs {y:?}")),
    };
    if a.keys().ne(b.keys()) {
        return Some(format!(
            "generated file sets differ: {:?} vs {:?}",
            a.keys(),
            b.keys()
        ));
    }
    for (name, ta) in &a {
        if ta != &b[name] {
            return Some(format!(
                "generated {name} differs:\n{}",
                super::first_diff(ta, &b[name])
            ));
        }
    }
    None
}
