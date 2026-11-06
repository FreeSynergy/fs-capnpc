//! Writer for schema nodes (struct, enum, interface, const, annotation, file).
//!
//! Pattern: Strategy — the node kind selects the writer branch.

use crate::request::{
    annotations::write_annotations,
    field::write_field,
    types::{write_brand_opt, write_type},
    values::write_value,
};
use crate::schema::*;
use crate::schema_capnp as sc;

pub(super) fn write_node(mut b: sc::node::Builder<'_>, n: &Node) {
    b.set_id(n.id);
    b.set_display_name(n.display_name.as_str());
    b.set_display_name_prefix_length(n.display_name_prefix_length);
    b.set_scope_id(n.scope_id);
    if let Some(nested) = &n.nested_nodes {
        let mut l = b.reborrow().init_nested_nodes(nested.len() as u32);
        for (i, (name, id)) in nested.iter().enumerate() {
            let mut e = l.reborrow().get(i as u32);
            e.set_name(name.as_str());
            e.set_id(*id);
        }
    }
    write_annotations(|c| b.reborrow().init_annotations(c), &n.annotations);
    if !n.parameters.is_empty() {
        let mut l = b.reborrow().init_parameters(n.parameters.len() as u32);
        for (i, p) in n.parameters.iter().enumerate() {
            l.reborrow().get(i as u32).set_name(p.as_str());
        }
    }
    b.set_is_generic(n.is_generic);
    b.set_start_byte(n.start_byte);
    b.set_end_byte(n.end_byte);

    match &n.kind {
        NodeKind::File => b.set_file(()),
        NodeKind::Struct(s) => {
            let mut sb = b.init_struct();
            sb.set_data_word_count(s.data_word_count);
            sb.set_pointer_count(s.pointer_count);
            sb.set_preferred_list_encoding(sc::ElementSize::InlineComposite);
            sb.set_is_group(s.is_group);
            sb.set_discriminant_count(s.discriminant_count);
            sb.set_discriminant_offset(s.discriminant_offset);
            if !s.fields.is_empty() {
                let mut fl = sb.init_fields(s.fields.len() as u32);
                for (i, f) in s.fields.iter().enumerate() {
                    write_field(fl.reborrow().get(i as u32), f);
                }
            }
        }
        NodeKind::Enum(es) => {
            let eb = b.init_enum();
            let mut l = eb.init_enumerants(es.len() as u32);
            for (i, e) in es.iter().enumerate() {
                let mut x = l.reborrow().get(i as u32);
                x.set_name(e.name.as_str());
                x.set_code_order(e.code_order);
                write_annotations(|c| x.reborrow().init_annotations(c), &e.annotations);
            }
        }
        NodeKind::Interface(iface) => {
            let mut ib = b.init_interface();
            {
                let mut l = ib
                    .reborrow()
                    .init_superclasses(iface.superclasses.len() as u32);
                for (i, s) in iface.superclasses.iter().enumerate() {
                    let mut x = l.reborrow().get(i as u32);
                    x.set_id(s.id);
                    write_brand_opt(|| x.reborrow().init_brand(), &s.brand);
                }
            }
            let mut l = ib.init_methods(iface.methods.len() as u32);
            for (i, m) in iface.methods.iter().enumerate() {
                let mut x = l.reborrow().get(i as u32);
                x.set_name(m.name.as_str());
                x.set_code_order(m.code_order);
                {
                    let mut ip = x
                        .reborrow()
                        .init_implicit_parameters(m.implicit_parameters.len() as u32);
                    for (j, p) in m.implicit_parameters.iter().enumerate() {
                        ip.reborrow().get(j as u32).set_name(p.as_str());
                    }
                }
                x.set_param_struct_type(m.param_struct_type);
                write_brand_opt(|| x.reborrow().init_param_brand(), &m.param_brand);
                x.set_result_struct_type(m.result_struct_type);
                write_brand_opt(|| x.reborrow().init_result_brand(), &m.result_brand);
                write_annotations(|c| x.reborrow().init_annotations(c), &m.annotations);
            }
        }
        NodeKind::Const { ty, value } => {
            let mut cb = b.init_const();
            write_type(cb.reborrow().init_type(), ty);
            write_value(cb.init_value(), value);
        }
        NodeKind::Annotation { ty, targets } => {
            let mut ab = b.init_annotation();
            write_type(ab.reborrow().init_type(), ty);
            ab.set_targets_file(targets.file);
            ab.set_targets_const(targets.constant);
            ab.set_targets_enum(targets.enumeration);
            ab.set_targets_enumerant(targets.enumerant);
            ab.set_targets_struct(targets.structure);
            ab.set_targets_field(targets.field);
            ab.set_targets_union(targets.union);
            ab.set_targets_group(targets.group);
            ab.set_targets_interface(targets.interface);
            ab.set_targets_method(targets.method);
            ab.set_targets_param(targets.param);
            ab.set_targets_annotation(targets.annotation);
        }
    }
}
