//! Struct translator: member schema creation and group finishing.
//!
//! Pattern: Template method — per-member steps that `traverse_*` drive.

use super::struct_state::*;
use crate::ids::generate_group_id;
use crate::schema::*;

impl StructTranslator {
    /// `MemberInfo::addMemberSchema` for member `m`'s node: returns (field index, member info index).
    pub(crate) fn add_member_schema(&mut self, m: usize) -> usize {
        let node_ref = self.members[m].node;
        let child_count = self.members[m].child_count as usize;
        let has_fields = !self.struct_of(node_ref).fields.is_empty();
        if !has_fields {
            if self.members[m].parent.is_some() {
                self.get_schema(m);
            }
            let placeholder = Field {
                name: String::new(),
                code_order: 0,
                annotations: vec![],
                discriminant_value: NO_DISCRIMINANT,
                kind: FieldKind::Group { type_id: 0 },
                ordinal: Ordinal::Implicit,
            };
            self.struct_of(node_ref).fields = vec![placeholder; child_count];
            self.info_mut(node_ref).members = Some(vec![MemberSourceInfo::default(); child_count]);
        }
        let i = self.members[m].child_initialized_count as usize;
        self.members[m].child_initialized_count += 1;
        i
    }

    /// `MemberInfo::getSchema`: returns (node ref, field index) of this member's field.
    pub(crate) fn get_schema(&mut self, m: usize) -> (NodeRef, usize) {
        let parent = self.members[m].parent.expect("root has no field schema");
        let pnode = self.members[parent].node;
        if let Some(i) = self.members[m].schema {
            return (pnode, i);
        }
        self.members[m].index = self.members[parent].child_initialized_count;
        let i = self.add_member_schema(parent);
        let mut disc = NO_DISCRIMINANT;
        if self.members[m].is_in_union {
            disc = self.members[parent].union_discriminant_count as u16;
            self.members[parent].union_discriminant_count += 1;
        }
        let (name, code_order, doc, start, end) = {
            let mi = &self.members[m];
            (
                mi.name.clone(),
                mi.code_order,
                mi.doc_comment.clone(),
                mi.start_byte,
                mi.end_byte,
            )
        };
        {
            let f = &mut self.struct_of(pnode).fields[i];
            f.discriminant_value = disc;
            f.name = name;
            f.code_order = code_order as u16;
        }
        {
            let info = &mut self.info_mut(pnode).members.as_mut().unwrap()[i];
            info.doc_comment = doc;
            info.start_byte = start;
            info.end_byte = end;
        }
        self.members[m].schema = Some(i);
        (pnode, i)
    }

    pub(crate) fn finish_group(&mut self, m: usize) {
        let node_ref = self.members[m].node;
        if let Some(u) = self.members[m].union_scope {
            self.layout.add_discriminant(u);
            let count = self.members[m].union_discriminant_count as u16;
            let off = self.layout.discriminant_offset(u).expect("discriminant");
            let s = self.struct_of(node_ref);
            s.discriminant_count = count;
            s.discriminant_offset = off;
        }
        if let Some(parent) = self.members[m].parent {
            let pnode = self.members[parent].node;
            let parent_id = self.node_ref(pnode).id;
            let index = {
                // Make sure the field exists (assigns the index).
                self.get_schema(m);
                self.members[m].index
            };
            let group_id = generate_group_id(parent_id, index as u16);
            {
                let n = self.node_mut(node_ref);
                n.id = group_id;
                n.scope_id = parent_id;
            }
            let (pn, fi) = self.get_schema(m);
            self.struct_of(pn).fields[fi].kind = FieldKind::Group { type_id: group_id };
            let doc = self.members[m].doc_comment.clone();
            let info = self.info_mut(node_ref);
            info.id = group_id;
            if doc.is_some() {
                info.doc_comment = doc;
            }
        }
    }
}
