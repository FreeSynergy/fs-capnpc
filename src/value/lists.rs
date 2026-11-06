//! List construction (`build_list`, `prim_list`).
//!
//! Pattern: Builder — assembles `ListVal`s element by element.

use super::convert::{dyn_to_ptr, prim_bits};
use super::dynval::*;
use super::vt::Vt;
use crate::schema::*;

impl<'a> Vt<'a> {
    pub(super) fn build_list(&mut self, elem: &Type, vals: Vec<Option<DynVal>>) -> ListVal {
        let n = vals.len();
        match elem {
            Type::Void => ListVal::Void(n as u32),
            Type::Bool => ListVal::Bits(
                vals.into_iter()
                    .map(|v| matches!(v, Some(DynVal::Bool(true))))
                    .collect(),
            ),
            Type::Int8 | Type::UInt8 => self.prim_list(elem, vals, 1),
            Type::Int16 | Type::UInt16 | Type::Enum { .. } => self.prim_list(elem, vals, 2),
            Type::Int32 | Type::UInt32 | Type::Float32 => self.prim_list(elem, vals, 4),
            Type::Int64 | Type::UInt64 | Type::Float64 => self.prim_list(elem, vals, 8),
            Type::Struct { type_id, .. } => {
                let node = self.node(*type_id);
                let (dw, pc) = node
                    .as_ref()
                    .and_then(|n| {
                        n.struct_node()
                            .map(|s| (s.data_word_count, s.pointer_count))
                    })
                    .unwrap_or((0, 0));
                let elems = vals
                    .into_iter()
                    .map(|v| match v {
                        Some(DynVal::Struct(_, sv)) => sv,
                        _ => StructVal {
                            data_words: dw,
                            pointer_count: pc,
                            data: vec![0; dw as usize * 8],
                            ptrs: vec![Ptr::Null; pc as usize],
                        },
                    })
                    .collect();
                ListVal::Structs {
                    data_words: dw,
                    pointer_count: pc,
                    elems,
                }
            }
            _ => ListVal::Pointers(
                vals.into_iter()
                    .map(|v| match v {
                        Some(d) => dyn_to_ptr(d),
                        None => Ptr::Null,
                    })
                    .collect(),
            ),
        }
    }

    pub(super) fn prim_list(
        &mut self,
        elem: &Type,
        vals: Vec<Option<DynVal>>,
        size: u8,
    ) -> ListVal {
        let values = vals
            .into_iter()
            .map(|v| match v {
                Some(d) => prim_bits(&d, elem),
                None => 0,
            })
            .collect();
        ListVal::Bytes { size, values }
    }
}
