//! Annotation application compilation.
//!
//! Pattern: Strategy — target-specific validation of each applied annotation.

use super::*;
use crate::ast::*;
use crate::compiler::*;
use crate::generics::ImplicitParams;
use crate::parser::expression_string;
use crate::schema::*;

/// Operation `compile_annotation_applications`.
pub fn compile_annotation_applications(
    c: &mut Compiler,
    ts: &mut Translator,
    annotations: &[AnnotationApplication],
    target: Target,
) -> Vec<Annotation> {
    let mut out = vec![];
    for a in annotations {
        let mut ann = Annotation {
            id: 0,
            value: Value::Void,
            brand: Brand::default(),
        };
        let name = &a.name;
        if let Some(decl) = compile_decl_expr(c, ts, name, &ImplicitParams::none()) {
            match decl.kind() {
                Some(DeclKind::Annotation) => {
                    let (id, brand) = decl.id_and_brand(c);
                    ann.id = id;
                    ann.brand = brand;
                    if let Some(anode) = c.resolve_bootstrap_schema(id)
                        && let NodeKind::Annotation { ty, targets } = &anode.kind
                    {
                        if !has_target(targets, target) {
                            c.add_error(
                                ts.module,
                                name.span,
                                format!(
                                    "'{}' cannot be applied to this kind of declaration.",
                                    expression_string(name)
                                ),
                            );
                        }
                        match &a.value {
                            None => {
                                if matches!(ty, Type::Void) {
                                    ann.value = Value::Void;
                                } else {
                                    c.add_error(
                                        ts.module,
                                        name.span,
                                        format!("'{}' requires a value.", expression_string(name)),
                                    );
                                    ann.value = default_default_value(ty);
                                }
                            }
                            Some(expr) => {
                                let bound = crate::value::subst(ty, &ann.brand);
                                ann.value = compile_bootstrap_value(c, ts, expr, &bound);
                            }
                        }
                    }
                }
                _ => c.add_error(
                    ts.module,
                    name.span,
                    format!("'{}' is not an annotation.", expression_string(name)),
                ),
            }
        }
        out.push(ann);
    }
    out
}
