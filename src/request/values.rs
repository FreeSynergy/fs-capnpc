//! Writer for constant and default `Value`s.
//!
//! Pattern: Visitor — dispatches on the value variant and writes it to the builder.

use crate::request::{encode::set_any_pointer, types::text_of};
use crate::schema::*;
use crate::schema_capnp as sc;

pub(super) fn write_value(mut b: sc::value::Builder<'_>, v: &Value) {
    match v {
        Value::Void => b.set_void(()),
        Value::Bool(x) => b.set_bool(*x),
        Value::Int8(x) => b.set_int8(*x),
        Value::Int16(x) => b.set_int16(*x),
        Value::Int32(x) => b.set_int32(*x),
        Value::Int64(x) => b.set_int64(*x),
        Value::UInt8(x) => b.set_uint8(*x),
        Value::UInt16(x) => b.set_uint16(*x),
        Value::UInt32(x) => b.set_uint32(*x),
        Value::UInt64(x) => b.set_uint64(*x),
        Value::Float32(x) => b.set_float32(*x),
        Value::Float64(x) => b.set_float64(*x),
        Value::Enum(x) => b.set_enum(*x),
        Value::Interface => b.set_interface(()),
        Value::Text(p) => match p {
            Ptr::Text(t) => b.set_text(text_of(t).as_str()),
            _ => b.set_null_variant(12),
        },
        Value::Data(p) => match p {
            Ptr::Data(d) => b.set_data(&d[..]),
            _ => b.set_null_variant(13),
        },
        Value::List(p) => match p {
            Ptr::Null => b.set_null_variant(14),
            _ => set_any_pointer(b.init_list(), p),
        },
        Value::Struct(p) => match p {
            Ptr::Null => b.set_null_variant(16),
            _ => set_any_pointer(b.init_struct(), p),
        },
        Value::AnyPointer(p) => match p {
            Ptr::Null => b.set_null_variant(18),
            _ => set_any_pointer(b.init_any_pointer(), p),
        },
        Value::Pending(_) => panic!("unresolved pending value"),
    }
}
