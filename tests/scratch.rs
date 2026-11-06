#[test]
fn builtin_ids() {
    let bytes = capnpc_embedded::CompileCommand::new().src_prefix("tests/schemas").file("tests/schemas/builtins.capnp").compile().unwrap();
    let r = capnp::serialize::read_message(&bytes[..], Default::default()).unwrap();
    let req: capnp_compiler::schema_capnp::code_generator_request::Reader = r.get_root().unwrap();
    let f = req.get_requested_files().unwrap().get(0);
    let ids: Vec<u64> = f.get_file_source_info().unwrap().get_identifiers().unwrap().iter().map(|i| match i.which().unwrap() { capnp_compiler::schema_capnp::code_generator_request::requested_file::file_source_info::identifier::Which::TypeId(t) => t, _ => 0 }).collect();
    println!("IDS {:?}", ids);
}
