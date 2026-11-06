//! Type and value compilation helpers (types, default values, constants).
//!
//! Pattern: Adapter — adapts AST expressions to schema `Type`/`Value` through the compiler.

use super::*;
use crate::ast::*;
use crate::compiler::*;
use crate::generics::{BrandedDecl, ImplicitParams};
use crate::schema::*;

/// Operation `compile_type`.
pub fn compile_type(
    c: &mut Compiler,
    ts: &Translator,
    source: &Expression,
    implicit: &ImplicitParams,
) -> Option<Type> {
    let decl = ts
        .local_brand
        .compile_decl_expression(c, source, ts.node_idx, implicit)?;
    decl.compile_as_type(c, ts.module)
}

pub(crate) fn compile_decl_expr(
    c: &mut Compiler,
    ts: &Translator,
    source: &Expression,
    implicit: &ImplicitParams,
) -> Option<BrandedDecl> {
    ts.local_brand
        .compile_decl_expression(c, source, ts.node_idx, implicit)
}

/// Operation `default_default_value`.
pub fn default_default_value(ty: &Type) -> Value {
    match ty {
        Type::Void => Value::Void,
        Type::Bool => Value::Bool(false),
        Type::Int8 => Value::Int8(0),
        Type::Int16 => Value::Int16(0),
        Type::Int32 => Value::Int32(0),
        Type::Int64 => Value::Int64(0),
        Type::UInt8 => Value::UInt8(0),
        Type::UInt16 => Value::UInt16(0),
        Type::UInt32 => Value::UInt32(0),
        Type::UInt64 => Value::UInt64(0),
        Type::Float32 => Value::Float32(0.0),
        Type::Float64 => Value::Float64(0.0),
        Type::Enum { .. } => Value::Enum(0),
        Type::Interface { .. } => Value::Interface,
        Type::Text => Value::Text(Ptr::Null),
        Type::Data => Value::Data(Ptr::Null),
        Type::Struct { .. } => Value::Struct(Ptr::Null),
        Type::List(_) => Value::List(Ptr::Null),
        Type::AnyPointer(_) => Value::AnyPointer(Ptr::Null),
    }
}

/// `compileBootstrapValue`: primitives are evaluated now, pointer values are deferred.
pub fn compile_bootstrap_value(
    c: &mut Compiler,
    ts: &mut Translator,
    source: &Expression,
    ty: &Type,
) -> Value {
    let default = default_default_value(ty);
    match ty {
        Type::List(_) | Type::Struct { .. } | Type::Interface { .. } | Type::AnyPointer(_) => {
            ts.unfinished.push(Unfinished {
                source: source.clone(),
                ty: ty.clone(),
                default,
            });
            Value::Pending(ts.unfinished.len() - 1)
        }
        _ => crate::value::compile_value(c, ts, source, ty, true).unwrap_or(default),
    }
}

pub(crate) fn compile_const(
    c: &mut Compiler,
    ts: &mut Translator,
    ty_expr: &Expression,
    value: &Expression,
) -> (Type, Value) {
    match compile_type(c, ts, ty_expr, &ImplicitParams::none()) {
        Some(t) => {
            let v = compile_bootstrap_value(c, ts, value, &t);
            (t, v)
        }
        None => (Type::Void, Value::Void),
    }
}
