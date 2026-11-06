#[test]
fn dump() {
    let bytes = capnpc_embedded::CompileCommand::new()
        .src_prefix("tests/schemas").file("tests/schemas/tiny.capnp").compile().unwrap();
    let reader = capnp::serialize::read_message(&bytes[..], Default::default()).unwrap();
    let req: capnp_compiler::schema_capnp::code_generator_request::Reader = reader.get_root().unwrap();
    println!("{:#?}", req);
}
