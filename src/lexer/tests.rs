//! Unit tests for the lexer.
//!
//! Pattern: Facade — tests exercise only the public `lex` / `lex_tokens` entry points.

use super::*;

fn kinds(src: &str) -> Vec<Tok> {
    lex_tokens(src.as_bytes())
        .unwrap()
        .into_iter()
        .map(|t| t.tok)
        .collect()
}

#[test]
fn identifiers_and_ops() {
    assert_eq!(
        kinds("foo @0x1f :Int32"),
        vec![
            Tok::Ident("foo".into()),
            Tok::Op("@".into()),
            Tok::Int(0x1f),
            Tok::Op(":".into()),
            Tok::Ident("Int32".into())
        ]
    );
}

#[test]
fn numbers() {
    assert_eq!(
        kinds("12 0 017 0x10 1.5 2e3 1e+2"),
        vec![
            Tok::Int(12),
            Tok::Int(0),
            Tok::Int(0o17),
            Tok::Int(16),
            Tok::Float(1.5),
            Tok::Float(2000.0),
            Tok::Float(100.0)
        ]
    );
}

#[test]
fn strings() {
    assert_eq!(
        kinds(r#""a\n\x41\101\"""#),
        vec![Tok::Str(b"a\nAA\"".to_vec())]
    );
    assert_eq!(kinds("`raw text\n"), vec![Tok::Str(b"raw text\n".to_vec())]);
    assert_eq!(
        kinds(r#"0x"01 ab ff""#),
        vec![Tok::Bin(vec![1, 0xab, 0xff])]
    );
}

#[test]
fn lists() {
    let t = lex_tokens(b"(a, b = 1,) [x]").unwrap();
    match &t[0].tok {
        Tok::Paren(items) => assert_eq!(items.len(), 2),
        _ => panic!(),
    }
    match &t[1].tok {
        Tok::Bracket(items) => assert_eq!(items.len(), 1),
        _ => panic!(),
    }
    match &lex_tokens(b"()").unwrap()[0].tok {
        Tok::Paren(items) => assert!(items.is_empty()),
        _ => panic!(),
    }
}

#[test]
fn statements_and_doc_comments() {
    let src = "# ignored\nstruct Foo {  # doc for Foo\n  # more\n  a @0 :Int32;  # doc a\n  b @1 :Int8;\n}\n";
    let st = lex(src.as_bytes(), "x").unwrap();
    assert_eq!(st.len(), 1);
    assert_eq!(st[0].doc_comment.as_deref(), Some("doc for Foo\nmore\n"));
    match &st[0].body {
        Body::Block(b) => {
            assert_eq!(b.len(), 2);
            assert_eq!(b[0].doc_comment.as_deref(), Some("doc a\n"));
            assert_eq!(b[1].doc_comment, None);
        }
        _ => panic!(),
    }
}

#[test]
fn trailing_block_comment() {
    let st = lex(b"struct A {\n a @0 :Int8;\n}\n# late doc\n", "x").unwrap();
    assert_eq!(st[0].doc_comment.as_deref(), Some("late doc\n"));
}

#[test]
fn errors() {
    assert!(lex(b"struct A {", "x").is_err());
    assert!(lex(b"foo bar", "x").is_err());
    assert!(lex(b"\"unterminated\n;", "x").is_err());
}
