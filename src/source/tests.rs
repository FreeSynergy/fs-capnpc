//! Tests for path evaluation and the loaders.
//!
//! Pattern: Strategy — the same assertions run against the in-memory and disk strategies.

use super::*;
use crate::pipeline::DefaultFrontend;

#[test]
fn eval() {
    let base = vec!["a".to_string(), "b".to_string()];
    assert_eq!(
        eval_components(&base, "../c.capnp"),
        Some(vec!["a".into(), "c.capnp".into()])
    );
    assert_eq!(eval_components(&base, "../../../x"), None);
    assert_eq!(
        eval_components(&[], "capnp/c++.capnp"),
        Some(vec!["capnp".into(), "c++.capnp".into()])
    );
}

#[test]
fn std_import() {
    let mut l = ModuleLoader::new();
    let mut errs = vec![];
    let m = l
        .load_module(&Dir::Std, &["capnp".into(), "c++.capnp".into()], &mut errs)
        .unwrap();
    assert!(errs.is_empty(), "{errs:?}");
    assert!(
        l.import_relative(m, "/capnp/stream.capnp", &mut errs)
            .is_some()
    );
}

#[test]
fn memory_relative_import() {
    let mut mem = MemoryLoader::new();
    mem.add(
        "/w/a.capnp",
        "@0xdbb9ad1f14bf0b36;\nusing B = import \"b.capnp\";\n",
    );
    mem.add("/w/b.capnp", "@0xdbb9ad1f14bf0b37;\nstruct S {}\n");
    let mut l = ModuleLoader::with(Box::new(mem), Box::new(DefaultFrontend::standard()));
    let dir = Dir::Disk("/w".into());
    let mut errs = vec![];
    let a = l.load_module(&dir, &["a.capnp".into()], &mut errs).unwrap();
    assert!(l.import_relative(a, "b.capnp", &mut errs).is_some());
    assert!(errs.is_empty(), "{errs:?}");
    assert!(
        l.load_module(&dir, &["missing.capnp".into()], &mut errs)
            .is_none()
    );
}
