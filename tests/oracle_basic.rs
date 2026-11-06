mod common;

#[test]
fn tiny() {
    common::assert_same(&["tests/schemas/tiny.capnp"], "tests/schemas", &[]);
}
