//! Oracle test: the smallest schema compiled by both compilers must give identical requests.
//!
//! Pattern: Golden Master — the official `capnp` compiler output is the expected value.

mod common;

#[test]
fn tiny() {
    common::assert_same(&["tests/schemas/tiny.capnp"], "tests/schemas", &[]);
}
