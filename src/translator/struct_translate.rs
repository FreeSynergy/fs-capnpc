//! Struct translator: the final pass that lays out fields and writes the struct node.
//!
//! Pattern: Template method — fixed sequence of finishing steps after traversal.

use super::struct_state::*;
use super::*;
use crate::ast::*;
use crate::compiler::*;
use crate::error::Span;
use crate::generics::ImplicitParams;
use crate::schema::*;

impl StructTranslator {
    pub(crate) fn translate_internal(
        &mut self,
        c: &mut Compiler,
        ts: &mut Translator,
        root: usize,
    ) {
        if !matches!(self.top.kind, NodeKind::Struct(_)) {
            self.top.kind = NodeKind::Struct(StructNode::default());
        }
        // stable sort by ordinal (multimap semantics)
        let mut order = std::mem::take(&mut self.members_by_ordinal);
        order.sort_by_key(|e| e.0);
        let mut dup = DupOrdinal::new();
        for (ordinal, m) in order.iter().copied() {
            if let Some(o) = self.members[m].ordinal.clone() {
                dup.check(c, ts.module, &o);
            }
            let (pn, fi) = self.get_schema(m);
            self.struct_of(pn).fields[fi].ordinal = Ordinal::Explicit(ordinal as u16);

            match self.members[m].kind {
                MKind::Field => {
                    let ty_expr = self.members[m].field_type.clone().expect("field type");
                    let implicit = ImplicitParams {
                        scope_id: self.implicit.scope_id,
                        params: self.implicit.params.clone(),
                    };
                    let ty = compile_type(c, ts, &ty_expr, &implicit);
                    let (ty, ok) = match ty {
                        Some(t) => (t, true),
                        None => (Type::Void, false),
                    };
                    let mut had_explicit = false;
                    let default_value;
                    if ok {
                        if let Some(dv) = self.members[m].field_default.clone() {
                            if self.members[m].is_param
                                && matches!(&dv.kind, ExprKind::RelativeName(n) if n == "null")
                            {
                                match ty {
                                    Type::Text | Type::Data | Type::List(_) | Type::Struct { .. } | Type::Interface { .. } | Type::AnyPointer(_) => {}
                                    _ => c.add_error(ts.module, dv.span, "Only pointer parameters can declare their default as 'null'."),
                                }
                                default_value = default_default_value(&ty);
                            } else {
                                default_value = compile_bootstrap_value(c, ts, &dv, &ty);
                            }
                            had_explicit = true;
                        } else {
                            default_value = default_default_value(&ty);
                        }
                    } else {
                        default_value = default_default_value(&ty);
                    }

                    // -1 void, -2 pointer
                    let lg: i32 = match &ty {
                        Type::Void => -1,
                        Type::Bool => 0,
                        Type::Int8 | Type::UInt8 => 3,
                        Type::Int16 | Type::UInt16 | Type::Enum { .. } => 4,
                        Type::Int32 | Type::UInt32 | Type::Float32 => 5,
                        Type::Int64 | Type::UInt64 | Type::Float64 => 6,
                        _ => -2,
                    };
                    let scope = self.members[m].field_scope;
                    let offset = if lg == -2 {
                        self.layout.add_pointer(scope)
                    } else if lg == -1 {
                        self.layout.add_void(scope);
                        0
                    } else {
                        self.layout.add_data(scope, lg as u32)
                    };
                    self.struct_of(pn).fields[fi].kind = FieldKind::Slot {
                        offset,
                        ty,
                        default_value,
                        had_explicit_default: had_explicit,
                    };
                }
                MKind::Union => {
                    let u = self.members[m].union_scope.unwrap();
                    if !self.layout.add_discriminant(u) {
                        if let Some(o) = &self.members[m].ordinal {
                            c.add_error(ts.module, o.span, "Union ordinal, if specified, must be greater than no more than one of its member ordinals (i.e. there can only be one field retroactively unionized).");
                        }
                    }
                }
                MKind::Group => panic!("Groups don't have ordinals."),
            }
        }

        self.finish_group(root);
        let all = self.all_members.clone();
        for m in all {
            let target;
            if self.members[m].is_param {
                target = Target::Param;
            } else {
                match self.members[m].kind {
                    MKind::Field => target = Target::Field,
                    MKind::Union => {
                        self.finish_group(m);
                        target = Target::Union;
                    }
                    MKind::Group => {
                        self.finish_group(m);
                        target = Target::Group;
                    }
                }
            }
            let anns = self.members[m].annotations.clone();
            let compiled = compile_annotation_applications(c, ts, &anns, target);
            let (pn, fi) = self.get_schema(m);
            self.struct_of(pn).fields[fi].annotations = compiled;
        }

        let (dw, pc) = (
            self.layout.data_word_count as u16,
            self.layout.pointer_count as u16,
        );
        {
            let s = self.struct_of(NodeRef::Top);
            s.data_word_count = dw;
            s.pointer_count = pc;
        }
        for g in &mut self.groups {
            if let NodeKind::Struct(s) = &mut g.node.kind {
                s.data_word_count = dw;
                s.pointer_count = pc;
            }
        }
        if let Some(e) = self.layout.error.take() {
            c.add_error(ts.module, Span::new(ts.wip.start_byte, ts.wip.end_byte), e);
        }
    }
}
