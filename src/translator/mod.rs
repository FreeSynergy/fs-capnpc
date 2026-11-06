//! Declaration -> schema node translation (counterpart of `node-translator.c++`).
//!
//! `translate_node` builds the "bootstrap" node (everything except values of non-primitive type,
//! which are recorded as pending); `finish_translator` evaluates the pending values.
//!
//! Pattern: Facade — this module re-exports the small per-concern translators as one API.

mod annotations;
mod duplicates;
mod enums;
mod finish;
mod interfaces;
mod node;
mod params;
mod state;
mod struct_schema;
mod struct_state;
mod struct_translate;
mod struct_traverse;
mod struct_union;
mod structs;
mod target;
mod values;

pub use annotations::*;
pub(crate) use duplicates::*;
pub(crate) use enums::*;
pub use finish::*;
pub use interfaces::*;
pub use node::*;
pub(crate) use params::*;
pub use state::*;
pub(crate) use struct_state::*;
pub(crate) use structs::*;
pub use target::*;
pub use values::*;
