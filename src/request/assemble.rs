//! Assembly of the full `CodeGeneratorRequest` from the compiler output.
//!
//! Pattern: Facade — one entry point that drives all the node, field and value writers.

use crate::compiler::CompileOutput;
use crate::request::{node::write_node, source_info::write_source_info};
use crate::schema_capnp as sc;
use capnp::message::{Builder as MsgBuilder, HeapAllocator};

type Msg = MsgBuilder<HeapAllocator>;

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
