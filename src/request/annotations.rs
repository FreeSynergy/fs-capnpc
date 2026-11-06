//! Writer for annotation lists.
//!
//! Pattern: Visitor — writes each annotation with its brand and value.

use crate::request::{types::write_brand_opt, values::write_value};
use crate::schema::*;
use crate::schema_capnp as sc;

pub(super) fn write_annotations<'a>(
    b: impl FnOnce(u32) -> capnp::struct_list::Builder<'a, sc::annotation::Owned>,
    anns: &[Annotation],
) {
    if anns.is_empty() {
        return;
    }
    let mut list = b(anns.len() as u32);
    for (i, a) in anns.iter().enumerate() {
        let mut ab = list.reborrow().get(i as u32);
        ab.set_id(a.id);
        write_value(ab.reborrow().init_value(), &a.value);
        write_brand_opt(|| ab.reborrow().init_brand(), &a.brand);
    }
}
