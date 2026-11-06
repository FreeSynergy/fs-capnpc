//! Struct declaration entry point.
//!
//! Pattern: Facade — `compile_struct` drives the `StructTranslator` passes.

use super::*;
use crate::ast::*;
use crate::compiler::*;
use crate::generics::ImplicitParams;
use crate::layout::{Layout, Scope};
use crate::schema::*;
use std::rc::Rc;

pub(crate) fn root_member(node: NodeRef) -> MemberInfo {
    MemberInfo {
        parent: None,
        code_order: 0,
        index: 0,
        child_count: 0,
        child_initialized_count: 0,
        union_discriminant_count: 0,
        is_in_union: false,
        name: String::new(),
        ordinal: None,
        kind: MKind::Group,
        is_param: false,
        field_type: None,
        field_default: None,
        annotations: vec![],
        start_byte: 0,
        end_byte: 0,
        doc_comment: None,
        schema: None,
        node,
        field_scope: Scope::Top,
        union_scope: None,
    }
}

pub(crate) fn compile_struct(c: &mut Compiler, ts: &mut Translator, members: &[Rc<Declaration>]) {
    let mut st = StructTranslator {
        layout: Layout::new(),
        members: vec![root_member(NodeRef::Top)],
        members_by_ordinal: vec![],
        all_members: vec![],
        top: ts.wip.clone(),
        top_info: ts.source_info.clone(),
        groups: vec![],
        implicit: ImplicitParams::none(),
    };
    st.top.kind = NodeKind::Struct(StructNode::default());
    st.traverse_top_or_group(c, ts, members, 0, Scope::Top);
    st.translate_internal(c, ts, 0);
    ts.wip = st.top;
    ts.source_info = st.top_info;
    ts.groups = st.groups;
}
