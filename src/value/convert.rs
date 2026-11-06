//! Conversion of evaluated `DynVal`s into final schema `Value`s.
//!
//! Pattern: Adapter — `DynVal` to `Value` by expected type.

use super::dynval::*;
use super::vt::Vt;
use crate::schema::*;

impl Vt<'_> {
    pub(super) fn convert_value(&mut self, v: DynVal, ty: &Type) -> Value {
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
            u64::from(f.to_bits())
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
        Type::Bool => u64::from(matches!(v, DynVal::Bool(true))),
        Type::Enum { .. } => match v {
            DynVal::Enum(_, e) => u64::from(*e),
            _ => 0,
        },
        _ => {
            let x: u64 = match v {
                DynVal::Int(i) => *i as u64,
                DynVal::UInt(u) => *u,
                DynVal::Float(f) => *f as i64 as u64,
                DynVal::Bool(b) => u64::from(*b),
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
        Value::Int8(x) => u64::from(*x as u8),
        Value::Int16(x) => u64::from(*x as u16),
        Value::Int32(x) => u64::from(*x as u32),
        Value::Int64(x) => *x as u64,
        Value::UInt8(x) => u64::from(*x),
        Value::UInt16(x) => u64::from(*x),
        Value::UInt32(x) => u64::from(*x),
        Value::UInt64(x) => *x,
        Value::Float32(x) => u64::from(x.to_bits()),
        Value::Float64(x) => x.to_bits(),
        Value::Enum(x) => u64::from(*x),
        Value::Bool(b) => u64::from(*b),
        _ => 0,
    }
}
