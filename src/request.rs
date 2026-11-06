//! Serialisation of the compiler output into a `CodeGeneratorRequest` (counterpart of the request
//! assembly in `capnp.c++`).
//!
//! Nodes are first built in scratch messages and then deep-copied into the request, exactly like the
//! official compiler does, which makes the allocation order (and thus the encoded bytes) match.

use crate::compiler::CompileOutput;
use crate::schema::*;
use crate::schema_capnp as sc;
use capnp::message::{Builder as MsgBuilder, HeapAllocator, ReaderOptions};

type Msg = MsgBuilder<HeapAllocator>;

// ------------------------------------------------------------------------------------------
// flat encoding of pointer value trees

struct Enc {
    words: Vec<u64>,
}

impl Enc {
    fn alloc(&mut self, n: usize) -> usize {
        let p = self.words.len();
        self.words.resize(p + n, 0);
        p
    }

    fn put_bytes(&mut self, pos: usize, bytes: &[u8]) {
        for (i, chunk) in bytes.chunks(8).enumerate() {
            let mut w = [0u8; 8];
            w[..chunk.len()].copy_from_slice(chunk);
            self.words[pos + i] = u64::from_le_bytes(w);
        }
    }

    fn ptr_word(slot: usize, target: usize, kind: u32, hi: u32) -> u64 {
        let off = (target as i64 - (slot as i64 + 1)) as i32;
        let lo = ((off as u32) << 2) | kind;
        (lo as u64) | ((hi as u64) << 32)
    }

    fn encode_struct_body(&mut self, sv: &StructVal) -> usize {
        let dw = sv.data_words as usize;
        let pc = sv.pointer_count as usize;
        let pos = self.alloc(dw + pc);
        self.put_bytes(pos, &sv.data);
        for (i, p) in sv.ptrs.iter().enumerate() {
            self.encode(p, pos + dw + i);
        }
        pos
    }

    fn encode(&mut self, p: &Ptr, slot: usize) {
        match p {
            Ptr::Null => {}
            Ptr::Struct(sv) => {
                let pos = self.encode_struct_body(sv);
                self.words[slot] = Self::ptr_word(
                    slot,
                    pos,
                    0,
                    sv.data_words as u32 | ((sv.pointer_count as u32) << 16),
                );
            }
            Ptr::Text(t) => {
                let mut b = t.clone();
                b.push(0);
                let pos = self.alloc(b.len().div_ceil(8));
                self.put_bytes(pos, &b);
                self.words[slot] = Self::ptr_word(slot, pos, 1, 2 | ((b.len() as u32) << 3));
            }
            Ptr::Data(d) => {
                let pos = self.alloc(d.len().div_ceil(8));
                self.put_bytes(pos, d);
                self.words[slot] = Self::ptr_word(slot, pos, 1, 2 | ((d.len() as u32) << 3));
            }
            Ptr::List(l) => match l {
                ListVal::Void(n) => {
                    let pos = self.words.len();
                    self.words[slot] = Self::ptr_word(slot, pos, 1, *n << 3);
                }
                ListVal::Bits(bits) => {
                    let pos = self.alloc(bits.len().div_ceil(64));
                    for (i, b) in bits.iter().enumerate() {
                        if *b {
                            self.words[pos + i / 64] |= 1u64 << (i % 64);
                        }
                    }
                    self.words[slot] = Self::ptr_word(slot, pos, 1, 1 | ((bits.len() as u32) << 3));
                }
                ListVal::Bytes { size, values } => {
                    let size = *size as usize;
                    let total = values.len() * size;
                    let pos = self.alloc(total.div_ceil(8));
                    let mut bytes = vec![];
                    for v in values {
                        bytes.extend_from_slice(&v.to_le_bytes()[..size]);
                    }
                    self.put_bytes(pos, &bytes);
                    let code = match size {
                        1 => 2,
                        2 => 3,
                        4 => 4,
                        _ => 5,
                    };
                    self.words[slot] =
                        Self::ptr_word(slot, pos, 1, code | ((values.len() as u32) << 3));
                }
                ListVal::Pointers(ps) => {
                    let pos = self.alloc(ps.len());
                    for (i, p) in ps.iter().enumerate() {
                        self.encode(p, pos + i);
                    }
                    self.words[slot] = Self::ptr_word(slot, pos, 1, 6 | ((ps.len() as u32) << 3));
                }
                ListVal::Structs {
                    data_words,
                    pointer_count,
                    elems,
                } => {
                    let per = *data_words as usize + *pointer_count as usize;
                    let pos = self.alloc(1 + elems.len() * per);
                    self.words[pos] = Self::ptr_word(pos, elems.len(), 0, 0) & 0xffff_ffff
                        | (((*data_words as u64) | ((*pointer_count as u64) << 16)) << 32);
                    // tag word: element count in the offset field
                    self.words[pos] = ((elems.len() as u64) << 2)
                        | (((*data_words as u64) | ((*pointer_count as u64) << 16)) << 32);
                    for (i, e) in elems.iter().enumerate() {
                        let base = pos + 1 + i * per;
                        self.put_bytes(base, &e.data);
                        for (j, p) in e.ptrs.iter().enumerate() {
                            self.encode(p, base + *data_words as usize + j);
                        }
                    }
                    self.words[slot] =
                        Self::ptr_word(slot, pos, 1, 7 | (((elems.len() * per) as u32) << 3));
                }
            },
        }
    }
}

/// Build a one-segment serialized message whose root is `ptr`.
fn flat_message(ptr: &Ptr) -> Vec<u8> {
    let mut e = Enc { words: vec![0] };
    e.encode(ptr, 0);
    let mut out = Vec::new();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(e.words.len() as u32).to_le_bytes());
    for w in &e.words {
        out.extend_from_slice(&w.to_le_bytes());
    }
    out
}

fn set_any_pointer(mut target: capnp::any_pointer::Builder<'_>, ptr: &Ptr) {
    let bytes = flat_message(ptr);
    let reader = capnp::serialize::read_message(&mut &bytes[..], ReaderOptions::new())
        .expect("flat message");
    let root: capnp::any_pointer::Reader = reader.get_root().expect("root");
    target
        .set_as::<capnp::any_pointer::Owned>(root)
        .expect("copy value");
}

// ------------------------------------------------------------------------------------------

fn text_of(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn write_type(mut b: sc::type_::Builder<'_>, t: &Type) {
    match t {
        Type::Void => b.set_void(()),
        Type::Bool => b.set_bool(()),
        Type::Int8 => b.set_int8(()),
        Type::Int16 => b.set_int16(()),
        Type::Int32 => b.set_int32(()),
        Type::Int64 => b.set_int64(()),
        Type::UInt8 => b.set_uint8(()),
        Type::UInt16 => b.set_uint16(()),
        Type::UInt32 => b.set_uint32(()),
        Type::UInt64 => b.set_uint64(()),
        Type::Float32 => b.set_float32(()),
        Type::Float64 => b.set_float64(()),
        Type::Text => b.set_text(()),
        Type::Data => b.set_data(()),
        Type::List(e) => {
            let l = b.init_list();
            write_type(l.init_element_type(), e);
        }
        Type::Enum { type_id, brand } => {
            let mut e = b.init_enum();
            e.set_type_id(*type_id);
            write_brand_opt(|| e.reborrow().init_brand(), brand);
        }
        Type::Struct { type_id, brand } => {
            let mut e = b.init_struct();
            e.set_type_id(*type_id);
            write_brand_opt(|| e.reborrow().init_brand(), brand);
        }
        Type::Interface { type_id, brand } => {
            let mut e = b.init_interface();
            e.set_type_id(*type_id);
            write_brand_opt(|| e.reborrow().init_brand(), brand);
        }
        Type::AnyPointer(ap) => {
            let a = b.init_any_pointer();
            match ap {
                AnyPointer::Unconstrained(u) => {
                    let mut un = a.init_unconstrained();
                    match u {
                        Unconstrained::AnyKind => un.set_any_kind(()),
                        Unconstrained::Struct => un.set_struct(()),
                        Unconstrained::List => un.set_list(()),
                        Unconstrained::Capability => un.set_capability(()),
                    }
                }
                AnyPointer::Parameter {
                    scope_id,
                    parameter_index,
                } => {
                    let mut p = a.init_parameter();
                    p.set_scope_id(*scope_id);
                    p.set_parameter_index(*parameter_index);
                }
                AnyPointer::ImplicitMethodParameter { parameter_index } => {
                    let mut p = a.init_implicit_method_parameter();
                    p.set_parameter_index(*parameter_index);
                }
            }
        }
    }
}

/// Write `brand` using `init` to create the brand struct, but only when it has scopes.
fn write_brand_opt<'a>(init: impl FnOnce() -> sc::brand::Builder<'a>, brand: &Brand) {
    if brand.scopes.is_empty() {
        return;
    }
    write_brand(init(), brand);
}

fn write_brand(b: sc::brand::Builder<'_>, brand: &Brand) {
    let mut scopes = b.init_scopes(brand.scopes.len() as u32);
    for (i, s) in brand.scopes.iter().enumerate() {
        let mut sb = scopes.reborrow().get(i as u32);
        sb.set_scope_id(s.scope_id);
        match &s.kind {
            BrandScopeKind::Inherit => sb.set_inherit(()),
            BrandScopeKind::Bind(bs) => {
                let mut list = sb.init_bind(bs.len() as u32);
                for (j, x) in bs.iter().enumerate() {
                    let mut bb = list.reborrow().get(j as u32);
                    match x {
                        Binding::Unbound => bb.set_unbound(()),
                        Binding::Type(t) => write_type(bb.init_type(), t),
                    }
                }
            }
        }
    }
}

fn write_value(mut b: sc::value::Builder<'_>, v: &Value) {
    match v {
        Value::Void => b.set_void(()),
        Value::Bool(x) => b.set_bool(*x),
        Value::Int8(x) => b.set_int8(*x),
        Value::Int16(x) => b.set_int16(*x),
        Value::Int32(x) => b.set_int32(*x),
        Value::Int64(x) => b.set_int64(*x),
        Value::UInt8(x) => b.set_uint8(*x),
        Value::UInt16(x) => b.set_uint16(*x),
        Value::UInt32(x) => b.set_uint32(*x),
        Value::UInt64(x) => b.set_uint64(*x),
        Value::Float32(x) => b.set_float32(*x),
        Value::Float64(x) => b.set_float64(*x),
        Value::Enum(x) => b.set_enum(*x),
        Value::Interface => b.set_interface(()),
        Value::Text(p) => match p {
            Ptr::Text(t) => b.set_text(text_of(t).as_str()),
            _ => b.set_null_variant(12),
        },
        Value::Data(p) => match p {
            Ptr::Data(d) => b.set_data(&d[..]),
            _ => b.set_null_variant(13),
        },
        Value::List(p) => match p {
            Ptr::Null => b.set_null_variant(14),
            _ => set_any_pointer(b.init_list(), p),
        },
        Value::Struct(p) => match p {
            Ptr::Null => b.set_null_variant(16),
            _ => set_any_pointer(b.init_struct(), p),
        },
        Value::AnyPointer(p) => match p {
            Ptr::Null => b.set_null_variant(18),
            _ => set_any_pointer(b.init_any_pointer(), p),
        },
        Value::Pending(_) => panic!("unresolved pending value"),
    }
}

fn write_annotations<'a>(
    b: impl FnOnce(u32) -> capnp::struct_list::Builder<'a, sc::annotation::Owned>,
    anns: &[Annotation],
) {
    if anns.is_empty() {
        return;
    }
    let mut list = b(anns.len() as u32);
    for (i, a) in anns.iter().enumerate() {
        let mut ab = list.reborrow().get(i as u32);
        ab.set_id(a.id);
        write_value(ab.reborrow().init_value(), &a.value);
        write_brand_opt(|| ab.reborrow().init_brand(), &a.brand);
    }
}

fn write_node(mut b: sc::node::Builder<'_>, n: &Node) {
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

fn write_field(mut f: sc::field::Builder<'_>, fld: &Field) {
    f.set_name(fld.name.as_str());
    f.set_code_order(fld.code_order);
    write_annotations(|c| f.reborrow().init_annotations(c), &fld.annotations);
    f.set_discriminant_value(fld.discriminant_value);
    match &fld.kind {
        FieldKind::Slot {
            offset,
            ty,
            default_value,
            had_explicit_default,
        } => {
            let mut s = f.reborrow().init_slot();
            s.set_offset(*offset);
            write_type(s.reborrow().init_type(), ty);
            write_value(s.reborrow().init_default_value(), default_value);
            s.set_had_explicit_default(*had_explicit_default);
        }
        FieldKind::Group { type_id } => {
            f.reborrow().init_group().set_type_id(*type_id);
        }
    }
    let mut o = f.init_ordinal();
    match fld.ordinal {
        Ordinal::Implicit => o.set_implicit(()),
        Ordinal::Explicit(x) => o.set_explicit(x),
    }
}

fn write_source_info(mut b: sc::node::source_info::Builder<'_>, si: &SourceInfo) {
    b.set_id(si.id);
    if let Some(d) = &si.doc_comment {
        b.set_doc_comment(d.as_str());
    }
    if let Some(ms) = &si.members {
        let mut l = b.reborrow().init_members(ms.len() as u32);
        for (i, m) in ms.iter().enumerate() {
            let mut x = l.reborrow().get(i as u32);
            if let Some(d) = &m.doc_comment {
                x.set_doc_comment(d.as_str());
            }
        }
    }
    b.set_start_byte(si.start_byte);
    b.set_end_byte(si.end_byte);
}

/// Build the `CodeGeneratorRequest` message for the compiler output.
pub fn build_request(out: &CompileOutput) -> Msg {
    let mut message = Msg::new_default();
    {
        let mut req = message.init_root::<sc::code_generator_request::Builder>();
        {
            let mut v = req.reborrow().get_capnp_version().expect("version");
            v.set_major(1);
            v.set_minor(4);
            v.set_micro(0);
        }
        {
            let mut nodes = req.reborrow().init_nodes(out.nodes.len() as u32);
            for (i, n) in out.nodes.iter().enumerate() {
                let mut scratch = Msg::new_default();
                {
                    let root = scratch.init_root::<sc::node::Builder>();
                    write_node(root, n);
                }
                let reader = scratch
                    .get_root_as_reader::<sc::node::Reader>()
                    .expect("scratch node");
                nodes.set_with_caveats(i as u32, reader).expect("copy node");
            }
        }
        {
            let mut infos = req
                .reborrow()
                .init_source_info(out.source_info.len() as u32);
            for (i, si) in out.source_info.iter().enumerate() {
                let mut scratch = Msg::new_default();
                {
                    let root = scratch.init_root::<sc::node::source_info::Builder>();
                    write_source_info(root, si);
                }
                let reader = scratch
                    .get_root_as_reader::<sc::node::source_info::Reader>()
                    .expect("scratch si");
                infos
                    .set_with_caveats(i as u32, reader)
                    .expect("copy source info");
            }
        }
        {
            let mut files = req
                .reborrow()
                .init_requested_files(out.requested_files.len() as u32);
            for (i, f) in out.requested_files.iter().enumerate() {
                let mut fb = files.reborrow().get(i as u32);
                fb.set_id(f.id);
                fb.set_filename(f.filename.as_str());
                {
                    let mut imports = fb.reborrow().init_imports(f.imports.len() as u32);
                    for (j, (id, name)) in f.imports.iter().enumerate() {
                        let mut ib = imports.reborrow().get(j as u32);
                        ib.set_id(*id);
                        ib.set_name(name.as_str());
                    }
                }
                let fsi = fb.init_file_source_info();
                let mut ids = fsi.init_identifiers(f.resolutions.len() as u32);
                for (j, r) in f.resolutions.iter().enumerate() {
                    let mut ib = ids.reborrow().get(j as u32);
                    ib.set_start_byte(r.start_byte);
                    ib.set_end_byte(r.end_byte);
                    ib.set_type_id(r.type_id);
                }
            }
        }
    }
    message
}

/// Serialise the request in the standard (unpacked) message format.
pub fn request_bytes(out: &CompileOutput) -> Vec<u8> {
    let msg = build_request(out);
    let mut buf = Vec::new();
    capnp::serialize::write_message(&mut buf, &msg).expect("serialise");
    buf
}
