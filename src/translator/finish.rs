//! Finishing pass: evaluates deferred values and patches them into the node.
//!
//! Pattern: Visitor — walks nodes and annotations replacing pending values.

use super::*;
use crate::compiler::*;
use crate::schema::*;

/// Operation `finish_translator`.
pub fn finish_translator(
    c: &mut Compiler,
    idx: NodeIdx,
    mut ts: Translator,
) -> (Translator, NodeSetRc) {
    let _ = idx;
    let mut results: Vec<Value> = vec![];
    let mut i = 0;
    while i < ts.unfinished.len() {
        let (src, ty, default) = {
            let u = &ts.unfinished[i];
            (u.source.clone(), u.ty.clone(), u.default.clone())
        };
        let v = crate::value::compile_value(c, &mut ts, &src, &ty, false).unwrap_or(default);
        results.push(v);
        i += 1;
    }
    patch_node(&mut ts.wip, &results);
    for g in &mut ts.groups {
        patch_node(&mut g.node, &results);
    }
    for p in &mut ts.param_structs {
        patch_node(&mut p.node, &results);
    }
    let set = ts.bootstrap_set();
    (ts, set)
}

pub(crate) fn patch_value(v: &mut Value, results: &[Value]) {
    if let Value::Pending(k) = v {
        *v = results[*k].clone();
    }
}

pub(crate) fn patch_annotations(anns: &mut [Annotation], results: &[Value]) {
    for a in anns {
        patch_value(&mut a.value, results);
    }
}

pub(crate) fn patch_node(n: &mut Node, results: &[Value]) {
    patch_annotations(&mut n.annotations, results);
    match &mut n.kind {
        NodeKind::Const { value, .. } => patch_value(value, results),
        NodeKind::Struct(s) => {
            for f in &mut s.fields {
                patch_annotations(&mut f.annotations, results);
                if let FieldKind::Slot { default_value, .. } = &mut f.kind {
                    patch_value(default_value, results);
                }
            }
        }
        NodeKind::Enum(es) => {
            for e in es {
                patch_annotations(&mut e.annotations, results);
            }
        }
        NodeKind::Interface(i) => {
            for m in &mut i.methods {
                patch_annotations(&mut m.annotations, results);
            }
        }
        _ => {}
    }
}
