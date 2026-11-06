//! Method parameter and result list translation into struct nodes.
//!
//! Pattern: Adapter — turns parameter lists into ordinary struct schema nodes.

use super::*;
use crate::ast::*;
use crate::compiler::*;
use crate::error::Span;
use crate::generics::{BrandedDecl, ImplicitParams};
use crate::ids::generate_method_params_id;
use crate::layout::{Layout, Scope};
use crate::parser::expression_string;
use crate::schema::*;

/// Translate a method parameter/result list into a struct node.
pub(crate) fn translate_params(
    c: &mut Compiler,
    ts: &mut Translator,
    params: &[Param],
    top: Node,
    top_info: SourceInfo,
    implicit: ImplicitParams,
) -> (Node, SourceInfo) {
    let mut st = StructTranslator {
        layout: Layout::new(),
        members: vec![root_member(NodeRef::Top)],
        members_by_ordinal: vec![],
        all_members: vec![],
        top,
        top_info,
        groups: vec![],
        implicit,
    };
    st.top.kind = NodeKind::Struct(StructNode::default());
    for (i, p) in params.iter().enumerate() {
        st.members[0].child_count += 1;
        st.members.push(MemberInfo {
            parent: Some(0),
            code_order: i as u32,
            index: 0,
            child_count: 0,
            child_initialized_count: 0,
            union_discriminant_count: 0,
            is_in_union: false,
            name: p.name.value.clone(),
            ordinal: None,
            kind: MKind::Field,
            is_param: true,
            field_type: Some(p.ty.clone()),
            field_default: p.default_value.clone(),
            annotations: p.annotations.clone(),
            start_byte: p.span.start,
            end_byte: p.span.end,
            doc_comment: None,
            schema: None,
            node: NodeRef::Top,
            field_scope: Scope::Top,
            union_scope: None,
        });
        let m = st.members.len() - 1;
        st.all_members.push(m);
        st.members_by_ordinal.push((i as u64, m));
    }
    st.translate_internal(c, ts, 0);
    (st.top, st.top_info)
}

pub(crate) fn compile_param_list(
    c: &mut Compiler,
    ts: &mut Translator,
    method_name: &str,
    ordinal: u16,
    is_results: bool,
    param_list: Option<&ParamList>,
    implicit: &[String],
) -> (Id, Brand) {
    let kind = param_list.map(|p| &p.kind);
    match kind {
        None => compile_named_param_list(
            c,
            ts,
            method_name,
            ordinal,
            is_results,
            &[],
            implicit,
        ),
        Some(ParamListKind::Named(ps)) => compile_named_param_list(
            c,
            ts,
            method_name,
            ordinal,
            is_results,
            ps,
            implicit,
        ),
        Some(ParamListKind::Type(t)) => {
            let ip = ImplicitParams {
                scope_id: 0,
                params: implicit.to_vec(),
            };
            if let Some(target) = compile_decl_expr(c, ts, t, &ip) {
                match target.kind() {
                    Some(DeclKind::Struct) => return target.id_and_brand(c),
                    Some(_) => c.add_error(
                        ts.module,
                        t.span,
                        format!("'{}' is not a struct type.", expression_string(t)),
                    ),
                    None => {
                        target.add_error(c, ts.module, "Cannot use generic parameter as whole input or output of a method. Instead, use a parameter/result list containing a field with this type.");
                        return (0, Brand::default());
                    }
                }
            }
            (0, Brand::default())
        }
        Some(ParamListKind::Stream) => {
            let span = param_list.unwrap().span;
            match c.resolve_import(ts.node_idx, "/capnp/stream.capnp") {
                Some(stream) => {
                    if c.resolve_member(stream.resolver, "StreamResult").is_none() {
                        c.add_error(ts.module, span, "The version of '/capnp/stream.capnp' found in your import path does not appear to be the official one; it is missing the declaration of StreamResult.");
                    }
                }
                None => c.add_error(ts.module, span, "A method declaration uses streaming, but '/capnp/stream.capnp' is not found in the import path. This is a standard file that should always be installed with the Cap'n Proto compiler."),
            }
            (STREAM_RESULT_ID, Brand::default())
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn compile_named_param_list(
    c: &mut Compiler,
    ts: &mut Translator,
    method_name: &str,
    ordinal: u16,
    is_results: bool,
    params: &[Param],
    implicit: &[String],
) -> (Id, Brand) {
    let type_name = format!(
        "{}{}",
        method_name,
        if is_results { "$Results" } else { "$Params" }
    );
    let id = generate_method_params_id(ts.wip.id, ordinal, is_results);
    let display = format!("{}.{}", ts.wip.display_name, type_name);
    let node = Node {
        id,
        display_name: display.clone(),
        display_name_prefix_length: (display.len() - type_name.len()) as u32,
        scope_id: 0,
        nested_nodes: None,
        annotations: vec![],
        parameters: vec![],
        is_generic: ts.wip.is_generic || !implicit.is_empty(),
        // The reference compiler (1.4.0) leaves locations and the source-info id of the
        // implicit parameter structs unset.
        start_byte: 0,
        end_byte: 0,
        kind: NodeKind::Struct(StructNode::default()),
    };
    let info = SourceInfo::default();
    let (mut node, info) = translate_params(
        c,
        ts,
        params,
        node,
        info,
        ImplicitParams {
            scope_id: id,
            params: implicit.to_vec(),
        },
    );
    let brand = ts.local_brand.push(id, implicit.len());
    if !implicit.is_empty() {
        let decls: Vec<BrandedDecl> = (0..implicit.len())
            .map(BrandedDecl::implicit_method_param)
            .collect();
        node.parameters = implicit.to_vec();
        // Mirrors the reference implementation, which discards the resulting scope.
        let _ = brand.set_params(
            c,
            decls,
            DeclKind::Struct,
            &Expression {
                kind: ExprKind::Unknown,
                span: Span::default(),
            },
        );
    }
    let b = brand.compile(c);
    ts.param_structs.push(AuxNode {
        node,
        source_info: info,
    });
    (id, b)
}
