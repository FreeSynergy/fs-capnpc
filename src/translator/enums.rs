//! Enum translation.
//!
//! Pattern: Template method — fixed steps to turn enumerant declarations into an enum node.

use super::*;
use crate::ast::*;
use crate::compiler::*;
use crate::schema::*;
use std::rc::Rc;

pub(crate) fn compile_enum(c: &mut Compiler, ts: &mut Translator, members: &[Rc<Declaration>]) {
    let mut enumerants: Vec<(u64, u32, Rc<Declaration>)> = vec![];
    let mut code_order = 0u32;
    for m in members {
        if matches!(m.body, DeclBody::Enumerant) {
            enumerants.push((m.ordinal().unwrap_or(0), code_order, m.clone()));
            code_order += 1;
        }
    }
    enumerants.sort_by_key(|e| e.0); // stable: equal ordinals keep declaration order
    let mut list = vec![];
    let mut infos = vec![];
    let mut dup = DupOrdinal::new();
    for (_, code_order, d) in &enumerants {
        if let DeclId::Ordinal(o) = &d.id {
            dup.check(c, ts.module, o);
        }
        infos.push(MemberSourceInfo {
            doc_comment: d.doc_comment.clone(),
            start_byte: d.span.start,
            end_byte: d.span.end,
        });
        let annotations = compile_annotation_applications(c, ts, &d.annotations, Target::Enumerant);
        list.push(Enumerant {
            name: d.name.value.clone(),
            code_order: *code_order as u16,
            annotations,
        });
    }
    ts.wip.kind = NodeKind::Enum(list);
    ts.source_info.members = Some(infos);
}
