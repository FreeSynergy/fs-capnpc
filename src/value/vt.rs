//! The value translator state `Vt` and the public `compile_value` entry point.
//!
//! Pattern: Facade — `compile_value` hides the `Vt` machinery behind one function.

use crate::ast::*;
use crate::compiler::*;
use crate::schema::*;
use crate::translator::Translator;
use std::rc::Rc;

/// Evaluate `source` against the expected type `ty` and encode it as a schema [`Value`].
pub fn compile_value(
    c: &mut Compiler,
    ts: &mut Translator,
    source: &Expression,
    ty: &Type,
    is_bootstrap: bool,
) -> Option<Value> {
    let mut vt = Vt {
        c,
        ts,
        is_bootstrap,
    };
    let v = vt.compile_value(source, ty)?;
    Some(vt.to_value(v, ty))
}

pub(super) struct Vt<'a> {
    pub(super) c: &'a mut Compiler,
    pub(super) ts: &'a mut Translator,
    pub(super) is_bootstrap: bool,
}

impl<'a> Vt<'a> {
    pub(super) fn err(&mut self, span: crate::error::Span, msg: impl Into<String>) {
        let m = self.ts.module;
        self.c.add_error(m, span, msg);
    }

    pub(super) fn node(&mut self, id: Id) -> Option<Rc<Node>> {
        self.c.resolve_bootstrap_schema(id)
    }

    pub(super) fn type_name(&mut self, ty: &Type) -> String {
        match ty {
            Type::Void => "Void".into(),
            Type::Bool => "Bool".into(),
            Type::Int8 => "Int8".into(),
            Type::Int16 => "Int16".into(),
            Type::Int32 => "Int32".into(),
            Type::Int64 => "Int64".into(),
            Type::UInt8 => "UInt8".into(),
            Type::UInt16 => "UInt16".into(),
            Type::UInt32 => "UInt32".into(),
            Type::UInt64 => "UInt64".into(),
            Type::Float32 => "Float32".into(),
            Type::Float64 => "Float64".into(),
            Type::Text => "Text".into(),
            Type::Data => "Data".into(),
            Type::List(e) => format!("List({})", self.type_name(e)),
            Type::Enum { type_id, .. }
            | Type::Struct { type_id, .. }
            | Type::Interface { type_id, .. } => match self.node(*type_id) {
                Some(n) => n.display_name[n.display_name_prefix_length as usize..].to_string(),
                None => "?".into(),
            },
            Type::AnyPointer(_) => "AnyPointer".into(),
        }
    }
}
