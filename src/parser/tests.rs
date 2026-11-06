//! Unit tests for the parser.
//!
//! Pattern: Facade — tests drive the parser through `parse_file` only.

use super::*;
use crate::ast::*;
use crate::error::Diagnostic;
use crate::lexer::lex;

fn parse(src: &str) -> (ParsedFile, Vec<Diagnostic>) {
    let st = lex(src.as_bytes(), "t.capnp").unwrap();
    parse_file(&st, "t.capnp", true)
}

#[test]
fn struct_with_fields_and_unions() {
    let (f, errs) = parse(
        "@0xdbb9ad1f14bf0b36;\nstruct P(T) {\n a @0 :Int32 = 5 $foo(1);\n u :union { x @1 :Text; y @2 :List(T); }\n g :group { h @3 :Bool; }\n union { z @4 :Void; w @5 :Void; }\n}\n",
    );
    assert!(errs.is_empty(), "{errs:?}");
    assert_eq!(f.root.uid(), Some(0xdbb9ad1f14bf0b36));
    let s = &f.root.nested[0];
    assert_eq!(s.name.value, "P");
    assert_eq!(s.parameters.len(), 1);
    assert_eq!(s.nested.len(), 4);
    assert!(matches!(s.nested[1].body, DeclBody::Union));
    assert_eq!(s.nested[1].nested.len(), 2);
    assert!(matches!(s.nested[2].body, DeclBody::Group));
    assert_eq!(s.nested[3].name.value, "");
    assert_eq!(s.nested[0].annotations.len(), 1);
}

#[test]
fn interface_methods() {
    let (f, errs) = parse(
        "@0xdbb9ad1f14bf0b36;\ninterface I extends(J, K(Text)) {\n  foo @0 (a :Int32, b :Text = \"x\") -> (r :Bool);\n  bar @1 [T] () -> stream;\n  baz @2 Foo -> Bar;\n}\n",
    );
    assert!(errs.is_empty(), "{errs:?}");
    let i = &f.root.nested[0];
    match &i.body {
        DeclBody::Interface { superclasses } => assert_eq!(superclasses.len(), 2),
        _ => panic!(),
    }
    assert_eq!(i.nested.len(), 3);
    assert_eq!(i.nested[1].parameters.len(), 1);
}

#[test]
fn annotation_decl_and_consts() {
    let (f, errs) = parse(
        "@0xdbb9ad1f14bf0b36;\nannotation foo @0xdbb9ad1f14bf0b37 (struct, field) :Text;\nconst x :Int32 = -5;\nconst y :List(Text) = [\"a\" \"b\", \"c\"];\nusing Foo = import \"foo.capnp\".Foo;\nusing import \"x.capnp\".Bar;\nenum E { a @0; b @1; }\n$import \"c\";\n",
    );
    assert!(errs.is_empty(), "{errs:?}");
    match &f.root.nested[0].body {
        DeclBody::Annotation { targets, .. } => {
            assert!(targets.structure && targets.field && !targets.file);
        }
        _ => panic!(),
    }
    assert_eq!(f.root.nested[4].name.value, "Bar");
    assert_eq!(f.root.annotations.len(), 1);
}

#[test]
fn errors_reported() {
    let (_, errs) = parse("@0xdbb9ad1f14bf0b36;\nstruct A { a @0 Int32; }\n");
    assert!(!errs.is_empty());
    let (_, errs) = parse("struct A {}\n");
    assert!(
        errs.iter()
            .any(|e| e.message.contains("does not declare an ID"))
    );
    let (_, errs) = parse("@0xdbb9ad1f14bf0b36;\nstruct A { u @0 :union { a @1 :Int8; } }\n");
    assert!(errs.iter().any(|e| e.message.contains("exclamation")));
}
