//! Conversion of evaluated `DynVal`s into final schema `Value`s.
//!
//! Pattern: Adapter — `DynVal` to `Value` by expected type.

use super::dynval::*;
use super::vt::Vt;
use crate::schema::*;

impl<'a> Vt<'a> {
    pub(super) fn to_value(&mut self, v: DynVal, ty: &Type) -> Value {
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

pub(super) fn dyn_to_ptr(v: DynVal) -> Ptr {
    match v {
        DynVal::Text(t) => Ptr::Text(t),
        DynVal::Data(d) => Ptr::Data(d),
        DynVal::List(_, l) => Ptr::List(l),
        DynVal::Struct(_, s) => Ptr::Struct(s),
        _ => Ptr::Null,
    }
}

/// The raw bits of a primitive value as stored in the given type.
pub(super) fn prim_bits(v: &DynVal, ty: &Type) -> u64 {
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

pub(super) fn default_bits(v: &Value) -> u64 {
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
