//! Writers for schema types and brands.
//!
//! Pattern: Visitor — recursive walk over `Type` and `Brand` values into builders.

use crate::schema::*;
use crate::schema_capnp as sc;

pub(super) fn text_of(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

pub(super) fn write_type(mut b: sc::type_::Builder<'_>, t: &Type) {
    match t {
        Type::Void => b.set_void(()),
        Type::Bool => b.set_bool(()),
        Type::Int8 => b.set_int8(()),
        Type::Int16 => b.set_int16(()),
        Type::Int32 => b.set_int32(()),
        Type::Int64 => b.set_int64(()),
        Type::UInt8 => b.set_uint8(()),
        Type::UInt16 => b.set_uint16(()),
        Type::UInt32 => b.set_uint32(()),
        Type::UInt64 => b.set_uint64(()),
        Type::Float32 => b.set_float32(()),
        Type::Float64 => b.set_float64(()),
        Type::Text => b.set_text(()),
        Type::Data => b.set_data(()),
        Type::List(e) => {
            let l = b.init_list();
            write_type(l.init_element_type(), e);
        }
        Type::Enum { type_id, brand } => {
            let mut e = b.init_enum();
            e.set_type_id(*type_id);
            write_brand_opt(|| e.reborrow().init_brand(), brand);
        }
        Type::Struct { type_id, brand } => {
            let mut e = b.init_struct();
            e.set_type_id(*type_id);
            write_brand_opt(|| e.reborrow().init_brand(), brand);
        }
        Type::Interface { type_id, brand } => {
            let mut e = b.init_interface();
            e.set_type_id(*type_id);
            write_brand_opt(|| e.reborrow().init_brand(), brand);
        }
        Type::AnyPointer(ap) => {
            let a = b.init_any_pointer();
            match ap {
                AnyPointer::Unconstrained(u) => {
                    let mut un = a.init_unconstrained();
                    match u {
                        Unconstrained::AnyKind => un.set_any_kind(()),
                        Unconstrained::Struct => un.set_struct(()),
                        Unconstrained::List => un.set_list(()),
                        Unconstrained::Capability => un.set_capability(()),
                    }
                }
                AnyPointer::Parameter {
                    scope_id,
                    parameter_index,
                } => {
                    let mut p = a.init_parameter();
                    p.set_scope_id(*scope_id);
                    p.set_parameter_index(*parameter_index);
                }
                AnyPointer::ImplicitMethodParameter { parameter_index } => {
                    let mut p = a.init_implicit_method_parameter();
                    p.set_parameter_index(*parameter_index);
                }
            }
        }
    }
}

/// Write `brand` using `init` to create the brand struct, but only when it has scopes.
pub(super) fn write_brand_opt<'a>(init: impl FnOnce() -> sc::brand::Builder<'a>, brand: &Brand) {
    if brand.scopes.is_empty() {
        return;
    }
    write_brand(init(), brand);
}

pub(super) fn write_brand(b: sc::brand::Builder<'_>, brand: &Brand) {
    let mut scopes = b.init_scopes(brand.scopes.len() as u32);
    for (i, s) in brand.scopes.iter().enumerate() {
        let mut sb = scopes.reborrow().get(i as u32);
        sb.set_scope_id(s.scope_id);
        match &s.kind {
            BrandScopeKind::Inherit => sb.set_inherit(()),
            BrandScopeKind::Bind(bs) => {
                let mut list = sb.init_bind(bs.len() as u32);
                for (j, x) in bs.iter().enumerate() {
                    let mut bb = list.reborrow().get(j as u32);
                    match x {
                        Binding::Unbound => bb.set_unbound(()),
                        Binding::Type(t) => write_type(bb.init_type(), t),
                    }
                }
            }
        }
    }
}
