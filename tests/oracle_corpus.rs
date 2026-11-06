//! Compile every schema in tests/schemas with both compilers and compare the requests.

mod common;
use std::path::Path;

fn walk(dir: &Path, out: &mut Vec<String>) {
    for e in std::fs::read_dir(dir).unwrap().filter_map(|e| e.ok()) {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x == "capnp") {
            out.push(p.to_string_lossy().into_owned());
        }
    }
}

fn corpus() -> Vec<String> {
    let mut v = vec![];
    walk(Path::new("tests/schemas"), &mut v);
    v.sort();
    v
}

#[test]
fn corpus_matches_official_compiler() {
    let only = std::env::var("CORPUS_ONLY").ok();
    let mut failures = vec![];
    let mut ok = 0;
    for f in corpus() {
        if let Some(o) = &only {
            if !f.contains(o.as_str()) {
                continue;
            }
        }
        let name = f.trim_start_matches("tests/schemas/").to_string();
        let rep = common::compare(&[&f], "tests/schemas", &["tests/schemas"]);
        if rep.differences.is_empty() && rep.bytes_identical {
            ok += 1;
            println!("OK   {name}");
        } else {
            let mut msg = format!("FAIL {name}");
            if rep.differences.is_empty() {
                msg.push_str(" (bytes differ, content equal)");
            }
            println!("{msg}");
            if only.is_some() || std::env::var("VERBOSE").is_ok() {
                for d in &rep.differences {
                    println!("{d}");
                }
            } else if let Some(d) = rep.differences.first() {
                println!("  {}", d.lines().next().unwrap_or(""));
            }
            failures.push(name);
        }
    }
    println!("{ok} identical, {} different", failures.len());
    assert!(failures.is_empty(), "differing: {failures:?}");
}
