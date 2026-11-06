//! Writer for struct fields.
//!
//! Pattern: Visitor — slot and group fields are written by variant.

use crate::request::{annotations::write_annotations, types::write_type, values::write_value};
use crate::schema::*;
use crate::schema_capnp as sc;

pub(super) fn write_field(mut f: sc::field::Builder<'_>, fld: &Field) {
    f.set_name(fld.name.as_str());
    f.set_code_order(fld.code_order);
    write_annotations(|c| f.reborrow().init_annotations(c), &fld.annotations);
    f.set_discriminant_value(fld.discriminant_value);
    match &fld.kind {
        FieldKind::Slot {
            offset,
            ty,
            default_value,
            had_explicit_default,
        } => {
            let mut s = f.reborrow().init_slot();
            s.set_offset(*offset);
            write_type(s.reborrow().init_type(), ty);
            write_value(s.reborrow().init_default_value(), default_value);
            s.set_had_explicit_default(*had_explicit_default);
        }
        FieldKind::Group { type_id } => {
            f.reborrow().init_group().set_type_id(*type_id);
        }
    }
    let mut o = f.init_ordinal();
    match fld.ordinal {
        Ordinal::Implicit => o.set_implicit(()),
        Ordinal::Explicit(x) => o.set_explicit(x),
    }
}
