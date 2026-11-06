//! Embedded copies of the standard schema files (`/capnp/*.capnp`).
//!
//! Pattern: Registry — fixed table from virtual path to file contents, shared by every loader.

/// Embedded standard schema files (`/capnp/*.capnp`).
pub const STD_FILES: &[(&str, &str)] = &[
    ("capnp/c++.capnp", include_str!("../../std/capnp/c++.capnp")),
    (
        "capnp/schema.capnp",
        include_str!("../../std/capnp/schema.capnp"),
    ),
    (
        "capnp/stream.capnp",
        include_str!("../../std/capnp/stream.capnp"),
    ),
    (
        "capnp/persistent.capnp",
        include_str!("../../std/capnp/persistent.capnp"),
    ),
    ("capnp/rpc.capnp", include_str!("../../std/capnp/rpc.capnp")),
    (
        "capnp/rpc-twoparty.capnp",
        include_str!("../../std/capnp/rpc-twoparty.capnp"),
    ),
    (
        "capnp/rust.capnp",
        include_str!("../../std/capnp/rust.capnp"),
    ),
];

/// Contents of the embedded standard file at `path` (components of `capnp/c++.capnp`).
#[must_use]
pub fn std_file(path: &[String]) -> Option<Vec<u8>> {
    let name = path.join("/");
    STD_FILES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, c)| c.as_bytes().to_vec())
}
