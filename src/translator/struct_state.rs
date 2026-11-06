//! Shared state of the struct translator: member records and node access helpers.
//!
//! Pattern: State — `StructTranslator` carries the traversal state of one struct translation.

use super::*;
use crate::ast::*;
use crate::generics::ImplicitParams;
use crate::layout::{Layout, Scope};
use crate::schema::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum NodeRef {
    Top,
    Group(usize),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MKind {
    Field,
    Union,
    Group,
}

pub(crate) struct MemberInfo {
    pub(crate) parent: Option<usize>,
    pub(crate) code_order: u32,
    pub(crate) index: u32,
    pub(crate) child_count: u32,
    pub(crate) child_initialized_count: u32,
    pub(crate) union_discriminant_count: u32,
    pub(crate) is_in_union: bool,
    pub(crate) name: String,
    pub(crate) ordinal: Option<Located<u64>>,
    pub(crate) kind: MKind,
    pub(crate) is_param: bool,
    pub(crate) field_type: Option<Expression>,
    pub(crate) field_default: Option<Expression>,
    pub(crate) annotations: Vec<AnnotationApplication>,
    pub(crate) start_byte: u32,
    pub(crate) end_byte: u32,
    pub(crate) doc_comment: Option<String>,
    pub(crate) schema: Option<usize>,
    pub(crate) node: NodeRef,
    pub(crate) field_scope: Scope,
    pub(crate) union_scope: Option<usize>,
}

pub(crate) struct StructTranslator {
    pub(crate) layout: Layout,
    pub(crate) members: Vec<MemberInfo>,
    pub(crate) members_by_ordinal: Vec<(u64, usize)>,
    pub(crate) all_members: Vec<usize>,
    pub(crate) top: Node,
    pub(crate) top_info: SourceInfo,
    pub(crate) groups: Vec<AuxNode>,
    pub(crate) implicit: ImplicitParams,
}

impl StructTranslator {
    pub(crate) fn node_mut(&mut self, r: NodeRef) -> &mut Node {
        match r {
            NodeRef::Top => &mut self.top,
            NodeRef::Group(i) => &mut self.groups[i].node,
        }
    }
    pub(crate) fn node_ref(&self, r: NodeRef) -> &Node {
        match r {
            NodeRef::Top => &self.top,
            NodeRef::Group(i) => &self.groups[i].node,
        }
    }
    pub(crate) fn info_mut(&mut self, r: NodeRef) -> &mut SourceInfo {
        match r {
            NodeRef::Top => &mut self.top_info,
            NodeRef::Group(i) => &mut self.groups[i].source_info,
        }
    }

    pub(crate) fn struct_of(&mut self, r: NodeRef) -> &mut StructNode {
        match &mut self.node_mut(r).kind {
            NodeKind::Struct(s) => s,
            _ => panic!("not a struct node"),
        }
    }

    pub(crate) fn new_member(
        &mut self,
        parent: usize,
        code_order: u32,
        decl: &Declaration,
        kind: MKind,
        field_scope: Scope,
        node: NodeRef,
        is_in_union: bool,
    ) -> usize {
        let (field_type, field_default) = match &decl.body {
            DeclBody::Field { ty, default_value } => (Some(ty.clone()), default_value.clone()),
            _ => (None, None),
        };
        self.members.push(MemberInfo {
            parent: Some(parent),
            code_order,
            index: 0,
            child_count: 0,
            child_initialized_count: 0,
            union_discriminant_count: 0,
            is_in_union,
            name: decl.name.value.clone(),
            ordinal: match &decl.id {
                DeclId::Ordinal(o) => Some(o.clone()),
                _ => None,
            },
            kind,
            is_param: false,
            field_type,
            field_default,
            annotations: decl.annotations.clone(),
            start_byte: decl.span.start,
            end_byte: decl.span.end,
            doc_comment: decl.doc_comment.clone(),
            schema: None,
            node,
            field_scope,
            union_scope: None,
        });
        self.members.len() - 1
    }

    pub(crate) fn new_group_node(&mut self, parent_node: NodeRef, decl: &Declaration) -> NodeRef {
        let (parent_name, parent_generic) = {
            let p = self.node_ref(parent_node);
            (p.display_name.clone(), p.is_generic)
        };
        let name = &decl.name.value;
        let display = format!("{parent_name}.{name}");
        let prefix = (display.len() - name.len()) as u32;
        let node = Node {
            id: 0,
            display_name: display,
            display_name_prefix_length: prefix,
            scope_id: 0,
            nested_nodes: None,
            annotations: vec![],
            parameters: vec![],
            is_generic: parent_generic,
            // capnp 1.4.0 leaves the locations of group nodes unset.
            start_byte: 0,
            end_byte: 0,
            kind: NodeKind::Struct(StructNode {
                is_group: true,
                ..Default::default()
            }),
        };
        let info = SourceInfo::default();
        self.groups.push(AuxNode {
            node,
            source_info: info,
        });
        NodeRef::Group(self.groups.len() - 1)
    }
}
