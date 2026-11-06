//! Interface translation: methods and their implicit parameter/result structs.
//!
//! Pattern: Template method — per-method steps over a shared parameter-list compiler.

use super::*;
use crate::ast::*;
use crate::compiler::*;
use crate::error::Span;
use crate::generics::ImplicitParams;
use crate::schema::*;
use std::rc::Rc;

pub(crate) fn compile_interface(
    c: &mut Compiler,
    ts: &mut Translator,
    superclasses: &[Expression],
    members: &[Rc<Declaration>],
) {
    let mut iface = InterfaceNode::default();
    for sc in superclasses {
        let mut s = Superclass {
            id: 0,
            brand: Brand::default(),
        };
        if let Some(decl) = compile_decl_expr(c, ts, sc, &ImplicitParams::none()) {
            match decl.kind() {
                Some(DeclKind::Interface) => {
                    let (id, brand) = decl.id_and_brand(c);
                    s.id = id;
                    s.brand = brand;
                }
                Some(_) => decl.add_error(c, ts.module, format!("'{decl}' is not an interface.")),
                None => decl.add_error(
                    c,
                    ts.module,
                    format!(
                        "'{decl}' is an unbound generic parameter. Currently we don't support extending these."
                    ),
                ),
            }
        }
        iface.superclasses.push(s);
    }

    let mut methods: Vec<(u64, u32, Rc<Declaration>)> = vec![];
    let mut code_order = 0u32;
    for m in members {
        if matches!(m.body, DeclBody::Method { .. }) {
            methods.push((m.ordinal().unwrap_or(0), code_order, m.clone()));
            code_order += 1;
        }
    }
    methods.sort_by_key(|e| e.0);
    let mut infos = vec![];
    let mut dup = DupOrdinal::new();
    for (_, code_order, md) in &methods {
        let DeclBody::Method { params, results } = &md.body else {
            unreachable!()
        };
        let ord_decl = match &md.id {
            DeclId::Ordinal(o) => o.clone(),
            _ => Located::new(0, Span::default()),
        };
        dup.check(c, ts.module, &ord_decl);
        let ordinal = ord_decl.value as u16;
        infos.push(MemberSourceInfo {
            doc_comment: md.doc_comment.clone(),
            start_byte: md.span.start,
            end_byte: md.span.end,
        });

        let implicit_names: Vec<String> = md.parameters.iter().map(|p| p.name.clone()).collect();
        if matches!(params.kind, ParamListKind::Stream) {
            c.add_error(
                ts.module,
                params.span,
                "'stream' can only appear after '->', not before.",
            );
        }
        let (param_id, param_brand) = compile_param_list(
            c,
            ts,
            &md.name.value,
            ordinal,
            false,
            Some(params),
            &implicit_names,
        );
        let (result_id, result_brand) = compile_param_list(
            c,
            ts,
            &md.name.value,
            ordinal,
            true,
            results.as_ref(),
            &implicit_names,
        );
        let annotations = compile_annotation_applications(c, ts, &md.annotations, Target::Method);
        iface.methods.push(Method {
            name: md.name.value.clone(),
            code_order: *code_order as u16,
            implicit_parameters: implicit_names,
            param_struct_type: param_id,
            param_brand,
            result_struct_type: result_id,
            result_brand,
            annotations,
        });
    }
    ts.wip.kind = NodeKind::Interface(iface);
    ts.source_info.members = Some(infos);
}

/// Definition `STREAM_RESULT_ID`.
pub const STREAM_RESULT_ID: u64 = 0x995f9a3377c0b16e;
