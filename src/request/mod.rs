//! Serialisation of the compiler output into a `CodeGeneratorRequest` (counterpart of the request
//! assembly in `capnp.c++`), split by concern.
//!
//! Pattern: Facade — re-exports `build_request` and `request_bytes`.

mod annotations;
mod assemble;
mod encode;
mod field;
mod node;
mod source_info;
mod types;
mod values;

pub use assemble::{build_request, request_bytes};
