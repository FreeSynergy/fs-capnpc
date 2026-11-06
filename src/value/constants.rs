//! Constant resolution and conversion of existing schema values back to `DynVal`.
//!
//! Pattern: Adapter — turns resolved schema `Value`s into the evaluator's `DynVal`.

use super::dynval::*;
use super::vt::Vt;
use crate::ast::*;
use crate::compiler::*;
use crate::generics::ImplicitParams;
use crate::parser::expression_string;
use crate::schema::*;

impl<'a> Vt<'a> {
    pub(super) fn resolve_constant(&mut self, source: &Expression) -> Option<DynVal> {
        let ts_brand = self.ts.local_brand.clone();
        let decl = ts_brand.compile_decl_expression(
            self.c,
            source,
            self.ts.node_idx,
            &ImplicitParams::none(),
        )?;
        if decl.kind() != Some(DeclKind::Const) {
            let m = self.ts.module;
            self.c.add_error(
                m,
                source.span,
                format!(
                    "'{}' does not refer to a constant.",
                    expression_string(source)
                ),
            );
            return None;
        }
        let (id, _brand) = decl.id_and_brand(self.c);
        let schema = self.c.resolve_bootstrap_schema(id)?;
        let proto = if self.is_bootstrap {
            schema
        } else {
            self.c.resolve_final_schema(id)?
        };
        let NodeKind::Const { ty, value } = &proto.kind else {
            return None;
        };
        if let ExprKind::RelativeName(name) = &source.kind {
            let parent = match self.c.resolve_bootstrap_schema(proto.scope_id) {
                Some(scope) => {
                    if matches!(scope.kind, NodeKind::File) {
                        String::new()
                    } else {
                        scope.display_name[scope.display_name_prefix_length as usize..].to_string()
                    }
                }
                None => return self.value_to_dyn(value, ty),
            };
            let m = self.ts.module;
            self.c.add_error(
                m,
                source.span,
                format!(
                    "Constant names must be qualified to avoid confusion.  Please replace '{}' with '{}.{}', if that's what you intended.",
                    expression_string(source),
                    parent,
                    name
                ),
            );
        }
        self.value_to_dyn(value, ty)
    }

    pub(super) fn value_to_dyn(&mut self, v: &Value, ty: &Type) -> Option<DynVal> {
        Some(match v {
            Value::Void => DynVal::Void,
            Value::Bool(b) => DynVal::Bool(*b),
            Value::Int8(x) => DynVal::Int(*x as i64),
            Value::Int16(x) => DynVal::Int(*x as i64),
            Value::Int32(x) => DynVal::Int(*x as i64),
            Value::Int64(x) => DynVal::Int(*x),
            Value::UInt8(x) => DynVal::UInt(*x as u64),
            Value::UInt16(x) => DynVal::UInt(*x as u64),
            Value::UInt32(x) => DynVal::UInt(*x as u64),
            Value::UInt64(x) => DynVal::UInt(*x),
            Value::Float32(x) => DynVal::Float(*x as f64),
            Value::Float64(x) => DynVal::Float(*x),
            Value::Text(p) => DynVal::Text(match p {
                Ptr::Text(t) => t.clone(),
                _ => vec![],
            }),
            Value::Data(p) => DynVal::Data(match p {
                Ptr::Data(t) => t.clone(),
                _ => vec![],
            }),
            Value::Enum(e) => match ty {
                Type::Enum { type_id, .. } => DynVal::Enum(*type_id, *e),
                _ => return None,
            },
            Value::List(p) => match p {
                Ptr::List(l) => DynVal::List(ty.clone(), l.clone()),
                _ => {
                    let Type::List(e) = ty else { return None };
                    let l = self.build_list(e, vec![]);
                    DynVal::List(ty.clone(), l)
                }
            },
            Value::Struct(p) => match (p, ty) {
                (Ptr::Struct(s), Type::Struct { type_id, .. }) => {
                    DynVal::Struct(*type_id, s.clone())
                }
                (_, Type::Struct { type_id, .. }) => {
                    let n = self.node(*type_id)?;
                    DynVal::Struct(*type_id, new_struct_val(&n))
                }
                _ => return None,
            },
            Value::Interface | Value::AnyPointer(_) | Value::Pending(_) => return None,
        })
    }
}
