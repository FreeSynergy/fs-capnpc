//! Constant / default value evaluation (the `ValueTranslator` part of `node-translator.c++`).
//!
//! Expressions are evaluated against an expected [`Type`] and encoded straight into the pointer
//! trees ([`Ptr`]) stored in schema `Value`s.

use crate::ast::*;
use crate::compiler::*;
use crate::generics::ImplicitParams;
use crate::parser::expression_string;
use crate::schema::*;
use crate::translator::{compile_type_for_value, Translator};
use std::rc::Rc;

#[derive(Clone, Debug)]
enum DynVal {
    Void,
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    Text(Vec<u8>),
    Data(Vec<u8>),
    List(Type, ListVal),
    Enum(Id, u16),
    Struct(Id, StructVal),
}

pub fn compile_value(c: &mut Compiler, ts: &mut Translator, source: &Expression, ty: &Type, is_bootstrap: bool) -> Option<Value> {
    let mut vt = Vt { c, ts, is_bootstrap };
    let v = vt.compile_value(source, ty)?;
    Some(vt.to_value(v, ty))
}

fn int_range(ty: &Type) -> Option<(i128, i128)> {
    Some(match ty {
        Type::Int8 => (i8::MIN as i128, i8::MAX as i128),
        Type::Int16 => (i16::MIN as i128, i16::MAX as i128),
        Type::Int32 => (i32::MIN as i128, i32::MAX as i128),
        Type::Int64 => (i64::MIN as i128, i64::MAX as i128),
        Type::UInt8 => (0, u8::MAX as i128),
        Type::UInt16 => (0, u16::MAX as i128),
        Type::UInt32 => (0, u32::MAX as i128),
        Type::UInt64 => (0, u64::MAX as i128),
        _ => return None,
    })
}

struct Vt<'a> {
    c: &'a mut Compiler,
    ts: &'a mut Translator,
    is_bootstrap: bool,
}

/// Replace type parameters in `ty` by the bindings in `brand`.
fn subst(ty: &Type, brand: &Brand) -> Type {
    match ty {
        Type::AnyPointer(AnyPointer::Parameter { scope_id, parameter_index }) => {
            for s in &brand.scopes {
                if s.scope_id == *scope_id {
                    if let BrandScopeKind::Bind(b) = &s.kind {
                        return match b.get(*parameter_index as usize) {
                            Some(Binding::Type(t)) => t.clone(),
                            Some(Binding::Unbound) | None => {
                                Type::AnyPointer(AnyPointer::Unconstrained(Unconstrained::AnyKind))
                            }
                        };
                    }
                }
            }
            ty.clone()
        }
        Type::List(e) => Type::List(Box::new(subst(e, brand))),
        Type::Struct { type_id, brand: b } => Type::Struct { type_id: *type_id, brand: subst_brand(b, brand) },
        Type::Enum { type_id, brand: b } => Type::Enum { type_id: *type_id, brand: subst_brand(b, brand) },
        Type::Interface { type_id, brand: b } => Type::Interface { type_id: *type_id, brand: subst_brand(b, brand) },
        _ => ty.clone(),
    }
}

fn subst_brand(b: &Brand, outer: &Brand) -> Brand {
    Brand {
        scopes: b
            .scopes
            .iter()
            .map(|s| BrandScope {
                scope_id: s.scope_id,
                kind: match &s.kind {
                    BrandScopeKind::Bind(v) => BrandScopeKind::Bind(
                        v.iter()
                            .map(|x| match x {
                                Binding::Type(t) => Binding::Type(subst(t, outer)),
                                Binding::Unbound => Binding::Unbound,
                            })
                            .collect(),
                    ),
                    BrandScopeKind::Inherit => BrandScopeKind::Inherit,
                },
            })
            .collect(),
    }
}

fn new_struct_val(n: &Node) -> StructVal {
    let (dw, pc) = match &n.kind {
        NodeKind::Struct(s) => (s.data_word_count, s.pointer_count),
        _ => (0, 0),
    };
    StructVal { data_words: dw, pointer_count: pc, data: vec![0; dw as usize * 8], ptrs: vec![Ptr::Null; pc as usize] }
}

impl<'a> Vt<'a> {
    fn err(&mut self, span: crate::error::Span, msg: impl Into<String>) {
        let m = self.ts.module;
        self.c.add_error(m, span, msg);
    }

    fn node(&mut self, id: Id) -> Option<Rc<Node>> {
        self.c.resolve_bootstrap_schema(id)
    }

    fn type_name(&mut self, ty: &Type) -> String {
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
            Type::Enum { type_id, .. } | Type::Struct { type_id, .. } | Type::Interface { type_id, .. } => {
                match self.node(*type_id) {
                    Some(n) => n.display_name[n.display_name_prefix_length as usize..].to_string(),
                    None => "?".into(),
                }
            }
            Type::AnyPointer(_) => "AnyPointer".into(),
        }
    }

    fn compile_value(&mut self, src: &Expression, ty: &Type) -> Option<DynVal> {
        if let Type::AnyPointer(AnyPointer::Parameter { .. } | AnyPointer::ImplicitMethodParameter { .. }) = ty {
            self.err(src.span, "Cannot interpret value because the type is a generic type parameter which is not yet bound. We don't know what type to expect here.");
            return None;
        }
        let mut result = self.compile_value_inner(src, ty)?;
        if self.matches_type(src, ty, &mut result) {
            return Some(result);
        }
        // If the expected type is a struct, try matching its first field.
        if let Type::Struct { type_id, brand } = ty {
            if let Some(n) = self.node(*type_id) {
                if let Some(sn) = n.struct_node() {
                    if let Some(f) = sn.fields.first() {
                        if let FieldKind::Slot { ty: fty, .. } = &f.kind {
                            let fty = subst(fty, brand);
                            if self.matches_type(src, &fty, &mut result) {
                                let mut sv = new_struct_val(&n);
                                let field = f.clone();
                                self.set_field(&mut sv, &n, &field, brand, result);
                                return Some(DynVal::Struct(*type_id, sv));
                            }
                        }
                    }
                }
            }
        }
        let name = self.type_name(ty);
        self.err(src.span, format!("Type mismatch; expected {name}."));
        None
    }

    fn matches_type(&mut self, src: &Expression, ty: &Type, result: &mut DynVal) -> bool {
        match result {
            DynVal::Void => matches!(ty, Type::Void),
            DynVal::Bool(_) => matches!(ty, Type::Bool),
            DynVal::Int(v) if *v < 0 => {
                let min = match ty {
                    Type::Float32 | Type::Float64 => i64::MIN as i128,
                    _ => match int_range(ty) {
                        Some((lo, _)) => lo,
                        None => return false,
                    },
                };
                if (*v as i128) < min {
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
                    Type::Float32 | Type::Float64 => u64::MAX as u128,
                    _ => match int_range(ty) {
                        Some((_, hi)) => hi as u128,
                        None => return false,
                    },
                };
                if v as u128 > max {
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

    fn compile_value_inner(&mut self, src: &Expression, ty: &Type) -> Option<DynVal> {
        match &src.kind {
            ExprKind::RelativeName(id) => {
                if let Type::Enum { type_id, .. } = ty {
                    if let Some(n) = self.node(*type_id) {
                        if let NodeKind::Enum(es) = &n.kind {
                            if let Some(i) = es.iter().position(|e| &e.name == id) {
                                return Some(DynVal::Enum(*type_id, i as u16));
                            }
                        }
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
            ExprKind::AbsoluteName(_) | ExprKind::Import(_) | ExprKind::Application { .. } | ExprKind::Member { .. } => {
                self.resolve_constant(src)
            }
            ExprKind::Embed(name) => {
                let data = match self.c.read_embed(self.ts.node_idx, name) {
                    Some(d) => d,
                    None => {
                        self.err(src.span, format!("Couldn't read file for embed: {name}"));
                        return None;
                    }
                };
                match ty {
                    Type::Text => Some(DynVal::Text(data)),
                    Type::Data => Some(DynVal::Data(data)),
                    Type::Struct { .. } => {
                        self.err(src.span, "Embedding a struct message is not supported by this compiler.");
                        None
                    }
                    _ => {
                        self.err(src.span, "Embeds can only be used when Text, Data, or a struct is expected.");
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

    fn build_list(&mut self, elem: &Type, vals: Vec<Option<DynVal>>) -> ListVal {
        let n = vals.len();
        match elem {
            Type::Void => ListVal::Void(n as u32),
            Type::Bool => ListVal::Bits(
                vals.into_iter().map(|v| matches!(v, Some(DynVal::Bool(true)))).collect(),
            ),
            Type::Int8 | Type::UInt8 => self.prim_list(elem, vals, 1),
            Type::Int16 | Type::UInt16 | Type::Enum { .. } => self.prim_list(elem, vals, 2),
            Type::Int32 | Type::UInt32 | Type::Float32 => self.prim_list(elem, vals, 4),
            Type::Int64 | Type::UInt64 | Type::Float64 => self.prim_list(elem, vals, 8),
            Type::Struct { type_id, .. } => {
                let node = self.node(*type_id);
                let (dw, pc) = node
                    .as_ref()
                    .and_then(|n| n.struct_node().map(|s| (s.data_word_count, s.pointer_count)))
                    .unwrap_or((0, 0));
                let elems = vals
                    .into_iter()
                    .map(|v| match v {
                        Some(DynVal::Struct(_, sv)) => sv,
                        _ => StructVal { data_words: dw, pointer_count: pc, data: vec![0; dw as usize * 8], ptrs: vec![Ptr::Null; pc as usize] },
                    })
                    .collect();
                ListVal::Structs { data_words: dw, pointer_count: pc, elems }
            }
            _ => ListVal::Pointers(
                vals.into_iter()
                    .map(|v| match v {
                        Some(d) => dyn_to_ptr(d),
                        None => Ptr::Null,
                    })
                    .collect(),
            ),
        }
    }

    fn prim_list(&mut self, elem: &Type, vals: Vec<Option<DynVal>>, size: u8) -> ListVal {
        let values = vals
            .into_iter()
            .map(|v| match v {
                Some(d) => prim_bits(&d, elem),
                None => 0,
            })
            .collect();
        ListVal::Bytes { size, values }
    }

    fn fill_struct_value(&mut self, sv: &mut StructVal, node: &Rc<Node>, brand: &Brand, params: &[ExprParam]) {
        for a in params {
            let Some(name) = &a.name else {
                self.err(a.value.span, "Missing field name.");
                continue;
            };
            let field = node.struct_node().and_then(|s| s.fields.iter().find(|f| f.name == name.value)).cloned();
            let Some(field) = field else {
                self.err(name.span, format!("Struct has no field named '{}'.", name.value));
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
                    let Some(gnode) = self.node(*type_id) else { continue };
                    if let ExprKind::Tuple(ps) = &a.value.kind {
                        // Setting a union group member sets the group's discriminant in the parent.
                        self.write_discriminant(sv, node, &field);
                        let ps = ps.clone();
                        self.fill_struct_value(sv, &gnode, brand, &ps);
                    } else {
                        self.write_discriminant(sv, node, &field);
                        let first = gnode.struct_node().and_then(|s| s.fields.first()).cloned();
                        let mut done = false;
                        if let Some(gf) = first {
                            if let FieldKind::Slot { ty: gty, .. } = &gf.kind {
                                let gty = subst(gty, brand);
                                if let Some(mut r) = self.compile_value_inner(&a.value, &Type::Struct { type_id: *type_id, brand: brand.clone() }) {
                                    if self.matches_type(&a.value, &gty, &mut r) {
                                        self.set_field(sv, &gnode, &gf, brand, r);
                                        done = true;
                                    }
                                } else {
                                    continue;
                                }
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

    fn write_discriminant(&mut self, sv: &mut StructVal, node: &Node, field: &Field) {
        if field.discriminant_value != NO_DISCRIMINANT {
            if let Some(s) = node.struct_node() {
                let off = s.discriminant_offset as usize * 2;
                if off + 2 <= sv.data.len() {
                    sv.data[off..off + 2].copy_from_slice(&field.discriminant_value.to_le_bytes());
                }
            }
        }
    }

    fn set_field(&mut self, sv: &mut StructVal, node: &Node, field: &Field, brand: &Brand, v: DynVal) {
        self.write_discriminant(sv, node, field);
        let FieldKind::Slot { offset, ty, default_value, .. } = &field.kind else { return };
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
            Type::Int8 | Type::UInt8 | Type::Int16 | Type::UInt16 | Type::Int32 | Type::UInt32 | Type::Int64
            | Type::UInt64 | Type::Float32 | Type::Float64 | Type::Enum { .. } => {
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

    fn resolve_constant(&mut self, source: &Expression) -> Option<DynVal> {
        let ts_brand = self.ts.local_brand.clone();
        let decl = ts_brand.compile_decl_expression(self.c, source, self.ts.node_idx, &ImplicitParams::none())?;
        if decl.kind() != Some(DeclKind::Const) {
            let m = self.ts.module;
            self.c.add_error(m, source.span, format!("'{}' does not refer to a constant.", expression_string(source)));
            return None;
        }
        let (id, _brand) = decl.id_and_brand(self.c);
        let schema = self.c.resolve_bootstrap_schema(id)?;
        let proto = if self.is_bootstrap {
            schema
        } else {
            self.c.resolve_final_schema(id)?
        };
        let NodeKind::Const { ty, value } = &proto.kind else { return None };
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

    fn value_to_dyn(&mut self, v: &Value, ty: &Type) -> Option<DynVal> {
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
                (Ptr::Struct(s), Type::Struct { type_id, .. }) => DynVal::Struct(*type_id, s.clone()),
                (_, Type::Struct { type_id, .. }) => {
                    let n = self.node(*type_id)?;
                    DynVal::Struct(*type_id, new_struct_val(&n))
                }
                _ => return None,
            },
            Value::Interface | Value::AnyPointer(_) | Value::Pending(_) => return None,
        })
    }

    fn to_value(&mut self, v: DynVal, ty: &Type) -> Value {
        match ty {
            Type::Void => Value::Void,
            Type::Bool => Value::Bool(matches!(v, DynVal::Bool(true))),
            Type::Int8 => Value::Int8(prim_bits(&v, ty) as u8 as i8),
            Type::Int16 => Value::Int16(prim_bits(&v, ty) as u16 as i16),
            Type::Int32 => Value::Int32(prim_bits(&v, ty) as u32 as i32),
            Type::Int64 => Value::Int64(prim_bits(&v, ty) as i64),
            Type::UInt8 => Value::UInt8(prim_bits(&v, ty) as u8),
            Type::UInt16 => Value::UInt16(prim_bits(&v, ty) as u16),
            Type::UInt32 => Value::UInt32(prim_bits(&v, ty) as u32),
            Type::UInt64 => Value::UInt64(prim_bits(&v, ty)),
            Type::Float32 => Value::Float32(f32::from_bits(prim_bits(&v, ty) as u32)),
            Type::Float64 => Value::Float64(f64::from_bits(prim_bits(&v, ty))),
            Type::Enum { .. } => Value::Enum(match v {
                DynVal::Enum(_, e) => e,
                _ => 0,
            }),
            Type::Text => Value::Text(dyn_to_ptr(v)),
            Type::Data => Value::Data(dyn_to_ptr(v)),
            Type::List(_) => Value::List(dyn_to_ptr(v)),
            Type::Struct { .. } => Value::Struct(dyn_to_ptr(v)),
            Type::Interface { .. } => Value::Interface,
            Type::AnyPointer(_) => Value::AnyPointer(dyn_to_ptr(v)),
        }
    }
}

fn dyn_to_ptr(v: DynVal) -> Ptr {
    match v {
        DynVal::Text(t) => Ptr::Text(t),
        DynVal::Data(d) => Ptr::Data(d),
        DynVal::List(_, l) => Ptr::List(l),
        DynVal::Struct(_, s) => Ptr::Struct(s),
        _ => Ptr::Null,
    }
}

/// The raw bits of a primitive value as stored in the given type.
fn prim_bits(v: &DynVal, ty: &Type) -> u64 {
    match ty {
        Type::Float32 => {
            let f = match v {
                DynVal::Float(f) => *f as f32,
                DynVal::Int(i) => *i as f32,
                DynVal::UInt(u) => *u as f32,
                _ => 0.0,
            };
            f.to_bits() as u64
        }
        Type::Float64 => {
            let f = match v {
                DynVal::Float(f) => *f,
                DynVal::Int(i) => *i as f64,
                DynVal::UInt(u) => *u as f64,
                _ => 0.0,
            };
            f.to_bits()
        }
        Type::Bool => matches!(v, DynVal::Bool(true)) as u64,
        Type::Enum { .. } => match v {
            DynVal::Enum(_, e) => *e as u64,
            _ => 0,
        },
        _ => {
            let x: u64 = match v {
                DynVal::Int(i) => *i as u64,
                DynVal::UInt(u) => *u,
                DynVal::Float(f) => *f as i64 as u64,
                DynVal::Bool(b) => *b as u64,
                _ => 0,
            };
            match ty {
                Type::Int8 | Type::UInt8 => x & 0xff,
                Type::Int16 | Type::UInt16 => x & 0xffff,
                Type::Int32 | Type::UInt32 => x & 0xffff_ffff,
                _ => x,
            }
        }
    }
}

fn default_bits(v: &Value) -> u64 {
    match v {
        Value::Int8(x) => *x as u8 as u64,
        Value::Int16(x) => *x as u16 as u64,
        Value::Int32(x) => *x as u32 as u64,
        Value::Int64(x) => *x as u64,
        Value::UInt8(x) => *x as u64,
        Value::UInt16(x) => *x as u64,
        Value::UInt32(x) => *x as u64,
        Value::UInt64(x) => *x,
        Value::Float32(x) => x.to_bits() as u64,
        Value::Float64(x) => x.to_bits(),
        Value::Enum(x) => *x as u64,
        Value::Bool(b) => *b as u64,
        _ => 0,
    }
}

#[allow(dead_code)]
fn _unused() {
    let _ = compile_type_for_value;
}
