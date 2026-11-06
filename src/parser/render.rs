//! Render expressions back to schema syntax for error messages.
//!
//! Pattern: Visitor — recursively walks an `Expression` producing text.

use crate::ast::*;

/// Render an expression back to schema syntax (used in error messages).
pub fn expression_string(e: &Expression) -> String {
    fn tuple(params: &[ExprParam]) -> String {
        let parts: Vec<String> = params
            .iter()
            .map(|p| match &p.name {
                Some(n) => format!("{} = {}", n.value, expression_string(&p.value)),
                None => expression_string(&p.value),
            })
            .collect();
        format!("( {} )", parts.join(", "))
    }
    match &e.kind {
        ExprKind::Unknown => "<parse error>".into(),
        ExprKind::PositiveInt(v) => v.to_string(),
        ExprKind::NegativeInt(v) => format!("-{v}"),
        ExprKind::Float(v) => format!("{v}"),
        ExprKind::String(s) => format!("\"{}\"", String::from_utf8_lossy(s)),
        ExprKind::Binary(b) => {
            let h: Vec<String> = b
                .iter()
                .map(|x| format!("{:x}{:x}", x % 16, x / 16))
                .collect();
            format!("0x\"{}\"", h.join(" "))
        }
        ExprKind::RelativeName(n) => n.clone(),
        ExprKind::AbsoluteName(n) => format!(".{n}"),
        ExprKind::Import(n) => format!("import \"{n}\""),
        ExprKind::Embed(n) => format!("embed \"{n}\""),
        ExprKind::List(l) => {
            let parts: Vec<String> = l.iter().map(expression_string).collect();
            format!("[ {} ]", parts.join(", "))
        }
        ExprKind::Tuple(t) => tuple(t),
        ExprKind::Application { function, params } => {
            format!("{}({})", expression_string(function), tuple(params))
        }
        ExprKind::Member { parent, name } => {
            format!("{}.{}", expression_string(parent), name.value)
        }
    }
}
