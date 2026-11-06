//! Entry point of node translation: header construction and declaration dispatch.
//!
//! Pattern: Facade — `translate_node` hides the per-kind translators behind one call.

use super::*;
use crate::ast::*;
use crate::compiler::*;
use crate::generics::{BrandScope, ImplicitParams};
use crate::schema::*;
use std::collections::BTreeMap;

/// Operation `translate_node`.
pub fn translate_node(c: &mut Compiler, idx: NodeIdx) -> Translator {
    let decl = c.decl(idx);
    let id = c.nodes[idx].id;
    let display = c.nodes[idx].display_name.clone();
    let mut prefix = 0u32;
    if let Some(dot) = display.rfind('.') {
        prefix = dot as u32 + 1;
    }
    if let Some(colon) = display.rfind(':')
        && colon as u32 > prefix
    {
        prefix = colon as u32 + 1;
    }
    let scope_id = c.nodes[idx].parent.map_or(0, |p| c.nodes[p].id);
    let nested: Vec<(String, Id)> = c.nodes[idx]
        .nested
        .iter()
        .map(|(n, ci)| (n.clone(), c.nodes[*ci].id))
        .collect();
    let module = c.nodes[idx].module;
    let local_brand = BrandScope::new_root(c, module, id, decl.parameters.len(), idx);
    let wip = Node {
        id,
        display_name: display,
        display_name_prefix_length: prefix,
        scope_id,
        nested_nodes: Some(nested),
        annotations: vec![],
        parameters: vec![],
        is_generic: false,
        start_byte: 0,
        end_byte: 0,
        kind: NodeKind::File,
    };
    let mut ts = Translator {
        node_idx: idx,
        module,
        decl: decl.clone(),
        wip,
        source_info: SourceInfo::default(),
        groups: vec![],
        param_structs: vec![],
        unfinished: vec![],
        local_brand,
    };
    compile_node(c, &mut ts, &decl);
    ts
}

pub(crate) fn compile_node(c: &mut Compiler, ts: &mut Translator, decl: &Declaration) {
    check_duplicate_names(
        c,
        ts.module,
        &decl.nested,
        decl_kind(decl),
        &mut BTreeMap::new(),
    );

    if !decl.parameters.is_empty() {
        ts.wip.parameters = decl.parameters.iter().map(|p| p.name.clone()).collect();
    }
    ts.wip.is_generic = ts.local_brand.is_generic();

    let target = match &decl.body {
        DeclBody::File => Target::File,
        DeclBody::Const { ty, value } => {
            let (t, v) = compile_const(c, ts, ty, value);
            ts.wip.kind = NodeKind::Const { ty: t, value: v };
            Target::Const
        }
        DeclBody::Annotation { ty, targets } => {
            let t = compile_type(c, ts, ty, &ImplicitParams::none()).unwrap_or(Type::Void);
            ts.wip.kind = NodeKind::Annotation {
                ty: t,
                targets: *targets,
            };
            Target::Annotation
        }
        DeclBody::Enum => {
            compile_enum(c, ts, &decl.nested);
            Target::Enum
        }
        DeclBody::Struct => {
            compile_struct(c, ts, &decl.nested);
            Target::Struct
        }
        DeclBody::Interface { superclasses } => {
            compile_interface(c, ts, superclasses, &decl.nested);
            Target::Interface
        }
        _ => panic!("This Declaration is not a node."),
    };
    ts.wip.start_byte = decl.span.start;
    ts.wip.end_byte = decl.span.end;
    ts.wip.annotations = compile_annotation_applications(c, ts, &decl.annotations, target);

    ts.source_info.id = ts.wip.id;
    ts.source_info.doc_comment.clone_from(&decl.doc_comment);
    ts.source_info.start_byte = decl.span.start;
    ts.source_info.end_byte = decl.span.end;
}
