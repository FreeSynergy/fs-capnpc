//! Public entry point: parse a file's statements into a root declaration.
//!
//! Pattern: Facade — `parse_file` hides the statement/token parsers behind one call.

use super::types::{Ctx, Level, Parsed};
use crate::ast::*;
use crate::error::{Diagnostic, Span};
use crate::lexer::Statement;

/// The parsed file: a root [`Declaration`] of kind [`DeclBody::File`].
pub struct ParsedFile {
    /// Root declaration of the file.
    pub root: std::rc::Rc<Declaration>,
}

fn generate_random_id() -> u64 {
    use std::io::Read;
    let mut buf = [0u8; 8];
    let ok = std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut buf))
        .is_ok();
    let v = if ok {
        u64::from_le_bytes(buf)
    } else {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0x1234_5678_9abc_def0, |d| d.as_nanos() as u64);
        t.wrapping_mul(0x9e37_79b9_7f4a_7c15)
    };
    v | (1u64 << 63)
}

/// Parse the statements of a file. Errors are appended to the returned list.
pub fn parse_file(
    statements: &[Statement],
    file: &str,
    require_id: bool,
) -> (ParsedFile, Vec<Diagnostic>) {
    let mut ctx = Ctx {
        file: file.to_string(),
        errors: vec![],
    };
    let mut decls: Vec<std::rc::Rc<Declaration>> = vec![];
    let mut annotations = vec![];
    let mut file_id: Option<Located<u64>> = None;
    let mut file_doc: Option<String> = None;

    for st in statements {
        let Some(parsed) = ctx.parse_statement(st, Level::File) else {
            continue;
        };
        match parsed {
            Parsed::NakedId(id, doc) => {
                if file_id.is_some() {
                    ctx.err(st.span, "File can only have one ID.");
                } else {
                    file_id = Some(id);
                    if doc.is_some() {
                        file_doc = doc;
                    }
                }
            }
            Parsed::NakedAnnotation(a) => annotations.push(a),
            Parsed::Decl(d) => decls.push(std::rc::Rc::new(d)),
        }
    }

    let id = if let Some(i) = file_id {
        i
    } else {
        let id = generate_random_id();
        if require_id && ctx.errors.is_empty() {
            ctx.err(
                Span::new(0, 0),
                format!(
                    "File does not declare an ID.  I've generated one for you.  Add this line to your file: @0x{id:x};"
                ),
            );
        }
        Located::new(id, Span::default())
    };

    let root = Declaration {
        name: Located::new(String::new(), Span::default()),
        id: DeclId::Uid(id),
        parameters: vec![],
        nested: decls,
        annotations,
        span: Span::default(),
        doc_comment: file_doc,
        body: DeclBody::File,
    };
    (
        ParsedFile {
            root: std::rc::Rc::new(root),
        },
        ctx.errors,
    )
}
