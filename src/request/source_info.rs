//! Writer for per-node source info (doc comments).
//!
//! Pattern: Adapter — maps `SourceInfo` onto the schema builder.

use crate::schema::*;
use crate::schema_capnp as sc;

pub(super) fn write_source_info(mut b: sc::node::source_info::Builder<'_>, si: &SourceInfo) {
    b.set_id(si.id);
    if let Some(d) = &si.doc_comment {
        b.set_doc_comment(d.as_str());
    }
    if let Some(ms) = &si.members {
        let mut l = b.reborrow().init_members(ms.len() as u32);
        for (i, m) in ms.iter().enumerate() {
            let mut x = l.reborrow().get(i as u32);
            if let Some(d) = &m.doc_comment {
                x.set_doc_comment(d.as_str());
            }
        }
    }
    b.set_start_byte(si.start_byte);
    b.set_end_byte(si.end_byte);
}
