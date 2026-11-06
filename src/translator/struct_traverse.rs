//! Struct translator: traversal of union, group and top-level declarations.
//!
//! Pattern: Visitor — walks member declarations and dispatches per declaration kind.

use super::struct_state::*;
use super::*;
use crate::ast::*;
use crate::compiler::*;
use crate::error::Span;
use crate::layout::Scope;
use std::rc::Rc;

impl StructTranslator {
    pub(crate) fn traverse_union(
        &mut self,
        c: &mut Compiler,
        ts: &Translator,
        decl: &Declaration,
        members: &[Rc<Declaration>],
        parent: usize,
        union_id: usize,
        code_order: &mut u32,
    ) {
        if members.len() < 2 {
            c.add_error(
                ts.module,
                decl.span,
                "Union must have at least two members.",
            );
        }
        for member in members {
            let mut ordinal: Option<u64> = None;
            let mut mi: Option<usize> = None;
            match &member.body {
                DeclBody::Field { .. } => {
                    self.members[parent].child_count += 1;
                    let g = self.layout.new_group(union_id);
                    let m = self.new_member(
                        parent,
                        *code_order,
                        member,
                        MKind::Field,
                        g,
                        NodeRef::Top,
                        true,
                    );
                    *code_order += 1;
                    self.all_members.push(m);
                    ordinal = member.ordinal();
                    mi = Some(m);
                }
                DeclBody::Union => {
                    if member.name.value.is_empty() {
                        c.add_error(
                            ts.module,
                            member.span,
                            "Unions cannot contain unnamed unions.",
                        );
                    } else {
                        self.members[parent].child_count += 1;
                        let g = self.layout.new_group(union_id);
                        let inner_union = self.layout.new_union(g);
                        let pnode = self.members[parent].node;
                        let gn = self.new_group_node(pnode, member);
                        let m =
                            self.new_member(parent, *code_order, member, MKind::Union, g, gn, true);
                        *code_order += 1;
                        self.all_members.push(m);
                        self.members[m].union_scope = Some(inner_union);
                        let mut sub = 0;
                        self.traverse_union(
                            c,
                            ts,
                            member,
                            &member.nested,
                            m,
                            inner_union,
                            &mut sub,
                        );
                        if let DeclId::Ordinal(o) = &member.id {
                            ordinal = Some(o.value);
                        }
                        mi = Some(m);
                    }
                }
                DeclBody::Group => {
                    self.members[parent].child_count += 1;
                    let g = self.layout.new_group(union_id);
                    let pnode = self.members[parent].node;
                    let gn = self.new_group_node(pnode, member);
                    let m = self.new_member(parent, *code_order, member, MKind::Group, g, gn, true);
                    *code_order += 1;
                    self.all_members.push(m);
                    self.traverse_group(c, ts, &member.nested, m, g);
                    mi = Some(m);
                }
                _ => {}
            }
            if let (Some(o), Some(m)) = (ordinal, mi) {
                self.members_by_ordinal.push((o, m));
            }
        }
    }

    pub(crate) fn traverse_group(
        &mut self,
        c: &mut Compiler,
        ts: &Translator,
        members: &[Rc<Declaration>],
        parent: usize,
        scope: Scope,
    ) {
        if members.is_empty() {
            let (s, e) = (
                self.members[parent].start_byte,
                self.members[parent].end_byte,
            );
            c.add_error(
                ts.module,
                Span::new(s, e),
                "Group must have at least one member.",
            );
        }
        self.traverse_top_or_group(c, ts, members, parent, scope);
    }

    pub(crate) fn traverse_top_or_group(
        &mut self,
        c: &mut Compiler,
        ts: &Translator,
        members: &[Rc<Declaration>],
        parent: usize,
        scope: Scope,
    ) {
        let mut code_order = 0u32;
        for member in members {
            let mut ordinal: Option<u64> = None;
            let mut mi: Option<usize> = None;
            match &member.body {
                DeclBody::Field { .. } => {
                    self.members[parent].child_count += 1;
                    let m = self.new_member(
                        parent,
                        code_order,
                        member,
                        MKind::Field,
                        scope,
                        NodeRef::Top,
                        false,
                    );
                    code_order += 1;
                    self.all_members.push(m);
                    ordinal = member.ordinal();
                    mi = Some(m);
                }
                DeclBody::Union => {
                    let union_id = self.layout.new_union(scope);
                    let mut independent = 0u32;
                    let target;
                    let use_parent_order;
                    if member.name.value.is_empty() {
                        target = parent;
                        use_parent_order = true;
                    } else {
                        self.members[parent].child_count += 1;
                        let pnode = self.members[parent].node;
                        let gn = self.new_group_node(pnode, member);
                        let m = self.new_member(
                            parent,
                            code_order,
                            member,
                            MKind::Union,
                            scope,
                            gn,
                            false,
                        );
                        code_order += 1;
                        self.all_members.push(m);
                        target = m;
                        use_parent_order = false;
                    }
                    self.members[target].union_scope = Some(union_id);
                    if use_parent_order {
                        self.traverse_union(
                            c,
                            ts,
                            member,
                            &member.nested,
                            target,
                            union_id,
                            &mut code_order,
                        );
                    } else {
                        self.traverse_union(
                            c,
                            ts,
                            member,
                            &member.nested,
                            target,
                            union_id,
                            &mut independent,
                        );
                    }
                    if let DeclId::Ordinal(o) = &member.id {
                        ordinal = Some(o.value);
                    }
                    mi = Some(target);
                }
                DeclBody::Group => {
                    self.members[parent].child_count += 1;
                    let pnode = self.members[parent].node;
                    let gn = self.new_group_node(pnode, member);
                    let m =
                        self.new_member(parent, code_order, member, MKind::Group, scope, gn, false);
                    code_order += 1;
                    self.all_members.push(m);
                    self.traverse_group(c, ts, &member.nested, m, scope);
                }
                _ => {}
            }
            if let (Some(o), Some(m)) = (ordinal, mi) {
                self.members_by_ordinal.push((o, m));
            }
        }
    }
}
