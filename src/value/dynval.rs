//! Dynamic intermediate value (`DynVal`) and primitive-bit helpers.
//!
//! Pattern: Adapter — values are evaluated into `DynVal` first, then adapted to schema `Value`s.

use crate::schema::*;

#[derive(Clone, Debug)]
pub(super) enum DynVal {
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

pub(super) fn int_range(ty: &Type) -> Option<(i128, i128)> {
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

pub(super) fn new_struct_val(n: &Node) -> StructVal {
    let (dw, pc) = match &n.kind {
        NodeKind::Struct(s) => (s.data_word_count, s.pointer_count),
        _ => (0, 0),
    };
    StructVal {
        data_words: dw,
        pointer_count: pc,
        data: vec![0; dw as usize * 8],
        ptrs: vec![Ptr::Null; pc as usize],
    }
}
