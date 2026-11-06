//! Inner expression evaluation: dispatch on the expression kind.
//!
//! Pattern: Strategy — evaluation strategy is selected by the expected `Type`.

use super::dynval::*;
use super::vt::Vt;
use crate::ast::*;
use crate::schema::*;

impl Vt<'_> {
    pub(super) fn compile_value_inner(&mut self, src: &Expression, ty: &Type) -> Option<DynVal> {
        match &src.kind {
            ExprKind::RelativeName(id) => {
                if let Type::Enum { type_id, .. } = ty {
                    if let Some(n) = self.node(*type_id)
                        && let NodeKind::Enum(es) = &n.kind
                        && let Some(i) = es.iter().position(|e| &e.name == id)
                    {
                        return Some(DynVal::Enum(*type_id, i as u16));
                    }
                } else {
                    match id.as_str() {
                        "void" => return Some(DynVal::Void),
                        "true" => return Some(DynVal::Bool(true)),
                        "false" => return Some(DynVal::Bool(false)),
                        "nan" => return Some(DynVal::Float(f64::NAN)),
                        "inf" => return Some(DynVal::Float(f64::INFINITY)),
                        _ => {}
                    }
                }
                self.resolve_constant(src)
            }
            ExprKind::AbsoluteName(_)
            | ExprKind::Import(_)
            | ExprKind::Application { .. }
            | ExprKind::Member { .. } => self.resolve_constant(src),
            ExprKind::Embed(name) => {
                let Some(data) = self.c.read_embed(self.ts.node_idx, name) else {
                    self.err(src.span, format!("Couldn't read file for embed: {name}"));
                    return None;
                };
                match ty {
                    Type::Text => Some(DynVal::Text(data)),
                    Type::Data => Some(DynVal::Data(data)),
                    Type::Struct { type_id, .. } => {
                        let decoded = super::embed::decode_root_struct(&data);
                        if decoded.is_none() {
                            self.err(
                                src.span,
                                "Embedded file is not a valid Cap'n Proto message.",
                            );
                        }
                        decoded.map(|sv| DynVal::Struct(*type_id, sv))
                    }
                    _ => {
                        self.err(
                            src.span,
                            "Embeds can only be used when Text, Data, or a struct is expected.",
                        );
                        None
                    }
                }
            }
            ExprKind::PositiveInt(v) => Some(DynVal::UInt(*v)),
            ExprKind::NegativeInt(v) => {
                if *v > (u64::MAX >> 1) + 1 {
                    self.err(src.span, "Integer is too big to be negative.");
                    None
                } else {
                    Some(DynVal::Int((*v as i64).wrapping_neg()))
                }
            }
            ExprKind::Float(f) => Some(DynVal::Float(*f)),
            ExprKind::String(s) => {
                if matches!(ty, Type::Data) {
                    Some(DynVal::Data(s.clone()))
                } else {
                    Some(DynVal::Text(s.clone()))
                }
            }
            ExprKind::Binary(b) => {
                if !matches!(ty, Type::Data) {
                    let n = self.type_name(ty);
                    self.err(src.span, format!("Type mismatch; expected {n}."));
                    return None;
                }
                Some(DynVal::Data(b.clone()))
            }
            ExprKind::List(items) => {
                let Type::List(elem) = ty else {
                    let n = self.type_name(ty);
                    self.err(src.span, format!("Type mismatch; expected {n}."));
                    return None;
                };
                let elem = (**elem).clone();
                let mut vals: Vec<Option<DynVal>> = vec![];
                for it in items {
                    vals.push(self.compile_value(it, &elem));
                }
                let list = self.build_list(&elem, vals);
                Some(DynVal::List(ty.clone(), list))
            }
            ExprKind::Tuple(params) => {
                let Type::Struct { type_id, brand } = ty else {
                    let n = self.type_name(ty);
                    self.err(src.span, format!("Type mismatch; expected {n}."));
                    return None;
                };
                let n = self.node(*type_id)?;
                let mut sv = new_struct_val(&n);
                self.fill_struct_value(&mut sv, &n, brand, params);
                Some(DynVal::Struct(*type_id, sv))
            }
            ExprKind::Unknown => None,
        }
    }
}
