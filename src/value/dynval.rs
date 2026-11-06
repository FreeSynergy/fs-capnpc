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
        Type::Int8 => (i128::from(i8::MIN), i128::from(i8::MAX)),
        Type::Int16 => (i128::from(i16::MIN), i128::from(i16::MAX)),
        Type::Int32 => (i128::from(i32::MIN), i128::from(i32::MAX)),
        Type::Int64 => (i128::from(i64::MIN), i128::from(i64::MAX)),
        Type::UInt8 => (0, i128::from(u8::MAX)),
        Type::UInt16 => (0, i128::from(u16::MAX)),
        Type::UInt32 => (0, i128::from(u32::MAX)),
        Type::UInt64 => (0, i128::from(u64::MAX)),
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
