//! Struct value construction: field filling, discriminants and field setters.
//!
//! Pattern: Builder — assembles `StructVal`s field by field.

use super::convert::{default_bits, dyn_to_ptr, prim_bits};
use super::dynval::*;
use super::vt::Vt;
use crate::ast::*;
use crate::schema::*;
use std::rc::Rc;

use super::subst::subst;

impl Vt<'_> {
    pub(super) fn fill_struct_value(
        &mut self,
        sv: &mut StructVal,
        node: &Rc<Node>,
        brand: &Brand,
        params: &[ExprParam],
    ) {
        for a in params {
            let Some(name) = &a.name else {
                self.err(a.value.span, "Missing field name.");
                continue;
            };
            let field = node
                .struct_node()
                .and_then(|s| s.fields.iter().find(|f| f.name == name.value))
                .cloned();
            let Some(field) = field else {
                self.err(
                    name.span,
                    format!("Struct has no field named '{}'.", name.value),
                );
                continue;
            };
            match &field.kind {
                FieldKind::Slot { ty, .. } => {
                    let fty = subst(ty, brand);
                    if let Some(v) = self.compile_value(&a.value, &fty) {
                        self.set_field(sv, node, &field, brand, v);
                    }
                }
                FieldKind::Group { type_id } => {
                    let Some(gnode) = self.node(*type_id) else {
                        continue;
                    };
                    if let ExprKind::Tuple(ps) = &a.value.kind {
                        // Setting a union group member sets the group's discriminant in the parent.
                        self.write_discriminant(sv, node, &field);
                        let ps = ps.clone();
                        self.fill_struct_value(sv, &gnode, brand, &ps);
                    } else {
                        self.write_discriminant(sv, node, &field);
                        let first = gnode.struct_node().and_then(|s| s.fields.first()).cloned();
                        let mut done = false;
                        if let Some(gf) = first
                            && let FieldKind::Slot { ty: gty, .. } = &gf.kind
                        {
                            let gty = subst(gty, brand);
                            if let Some(mut r) = self.compile_value_inner(
                                &a.value,
                                &Type::Struct {
                                    type_id: *type_id,
                                    brand: brand.clone(),
                                },
                            ) {
                                if self.matches_type(&a.value, &gty, &mut r) {
                                    self.set_field(sv, &gnode, &gf, brand, r);
                                    done = true;
                                }
                            } else {
                                continue;
                            }
                        }
                        if !done {
                            self.err(a.value.span, "Type mismatch; expected group.");
                        }
                    }
                }
            }
        }
    }

    pub(super) fn write_discriminant(&mut self, sv: &mut StructVal, node: &Node, field: &Field) {
        if field.discriminant_value != NO_DISCRIMINANT
            && let Some(s) = node.struct_node()
        {
            let off = s.discriminant_offset as usize * 2;
            if off + 2 <= sv.data.len() {
                sv.data[off..off + 2].copy_from_slice(&field.discriminant_value.to_le_bytes());
            }
        }
    }

    pub(super) fn set_field(
        &mut self,
        sv: &mut StructVal,
        node: &Node,
        field: &Field,
        brand: &Brand,
        v: DynVal,
    ) {
        self.write_discriminant(sv, node, field);
        let FieldKind::Slot {
            offset,
            ty,
            default_value,
            ..
        } = &field.kind
        else {
            return;
        };
        let ty = subst(ty, brand);
        let offset = *offset as usize;
        match &ty {
            Type::Void => {}
            Type::Bool => {
                let mut b = matches!(v, DynVal::Bool(true));
                if matches!(default_value, Value::Bool(true)) {
                    b = !b;
                }
                let (byte, bit) = (offset / 8, offset % 8);
                if byte < sv.data.len() {
                    if b {
                        sv.data[byte] |= 1 << bit;
                    } else {
                        sv.data[byte] &= !(1 << bit);
                    }
                }
            }
            Type::Int8
            | Type::UInt8
            | Type::Int16
            | Type::UInt16
            | Type::Int32
            | Type::UInt32
            | Type::Int64
            | Type::UInt64
            | Type::Float32
            | Type::Float64
            | Type::Enum { .. } => {
                let (size, bits) = match &ty {
                    Type::Int8 | Type::UInt8 => (1, prim_bits(&v, &ty)),
                    Type::Int16 | Type::UInt16 | Type::Enum { .. } => (2, prim_bits(&v, &ty)),
                    Type::Int32 | Type::UInt32 | Type::Float32 => (4, prim_bits(&v, &ty)),
                    _ => (8, prim_bits(&v, &ty)),
                };
                let dflt = default_bits(default_value);
                let x = bits ^ dflt;
                let start = offset * size;
                if start + size <= sv.data.len() {
                    sv.data[start..start + size].copy_from_slice(&x.to_le_bytes()[..size]);
                }
            }
            _ => {
                if offset < sv.ptrs.len() {
                    sv.ptrs[offset] = dyn_to_ptr(v);
                }
            }
        }
    }

    // ---- constants ---------------------------------------------------------------------
}
