//! Expression evaluation against an expected type.
//!
//! Pattern: Strategy — evaluation strategy is selected by the expected `Type`.

use super::dynval::*;
use super::vt::Vt;
use crate::ast::*;
use crate::schema::*;

use super::subst::subst;

impl Vt<'_> {
    pub(super) fn compile_value(&mut self, src: &Expression, ty: &Type) -> Option<DynVal> {
        if let Type::AnyPointer(
            AnyPointer::Parameter { .. } | AnyPointer::ImplicitMethodParameter { .. },
        ) = ty
        {
            self.err(src.span, "Cannot interpret value because the type is a generic type parameter which is not yet bound. We don't know what type to expect here.");
            return None;
        }
        let mut result = self.compile_value_inner(src, ty)?;
        if self.matches_type(src, ty, &mut result) {
            return Some(result);
        }
        // If the expected type is a struct, try matching its first field.
        if let Type::Struct { type_id, brand } = ty
            && let Some(n) = self.node(*type_id)
            && let Some(sn) = n.struct_node()
            && let Some(f) = sn.fields.first()
            && let FieldKind::Slot { ty: fty, .. } = &f.kind
        {
            let fty = subst(fty, brand);
            if self.matches_type(src, &fty, &mut result) {
                let mut sv = new_struct_val(&n);
                let field = f.clone();
                self.set_field(&mut sv, &n, &field, brand, result);
                return Some(DynVal::Struct(*type_id, sv));
            }
        }
        let name = self.type_name(ty);
        self.err(src.span, format!("Type mismatch; expected {name}."));
        None
    }

    pub(super) fn matches_type(
        &mut self,
        src: &Expression,
        ty: &Type,
        result: &mut DynVal,
    ) -> bool {
        match result {
            DynVal::Void => matches!(ty, Type::Void),
            DynVal::Bool(_) => matches!(ty, Type::Bool),
            DynVal::Int(v) if *v < 0 => {
                let min = match ty {
                    Type::Float32 | Type::Float64 => i128::from(i64::MIN),
                    _ => match int_range(ty) {
                        Some((lo, _)) => lo,
                        None => return false,
                    },
                };
                if i128::from(*v) < min {
                    self.err(src.span, "Integer value out of range.");
                    *v = min as i64;
                }
                true
            }
            DynVal::Int(_) | DynVal::UInt(_) => {
                let v: u64 = match result {
                    DynVal::Int(v) => *v as u64,
                    DynVal::UInt(v) => *v,
                    _ => unreachable!(),
                };
                let max: u128 = match ty {
                    Type::Float32 | Type::Float64 => u128::from(u64::MAX),
                    _ => match int_range(ty) {
                        Some((_, hi)) => hi as u128,
                        None => return false,
                    },
                };
                if u128::from(v) > max {
                    self.err(src.span, "Integer value out of range.");
                    *result = DynVal::UInt(max as u64);
                }
                true
            }
            DynVal::Float(_) => matches!(ty, Type::Float32 | Type::Float64),
            DynVal::Text(_) => matches!(ty, Type::Text),
            DynVal::Data(_) => matches!(ty, Type::Data),
            DynVal::List(lt, _) => match ty {
                Type::List(_) => lt == ty,
                Type::AnyPointer(AnyPointer::Unconstrained(k)) => {
                    matches!(k, Unconstrained::AnyKind | Unconstrained::List)
                }
                _ => false,
            },
            DynVal::Enum(id, _) => matches!(ty, Type::Enum { type_id, .. } if type_id == id),
            DynVal::Struct(id, _) => match ty {
                Type::Struct { type_id, .. } => type_id == id,
                Type::AnyPointer(AnyPointer::Unconstrained(k)) => {
                    matches!(k, Unconstrained::AnyKind | Unconstrained::Struct)
                }
                _ => false,
            },
        }
    }

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
                    Type::Struct { type_id, .. } => match super::embed::decode_root_struct(&data) {
                        Some(sv) => Some(DynVal::Struct(*type_id, sv)),
                        None => {
                            self.err(
                                src.span,
                                "Embedded file is not a valid Cap'n Proto message.",
                            );
                            None
                        }
                    },
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
