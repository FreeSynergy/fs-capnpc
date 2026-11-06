//! The compiler core (counterpart of `compiler.c++`), split by concern.
//!
//! Pattern: Facade — re-exports the compiler API from its small concern modules.

mod core;
mod driver;
mod errors;
mod imports;
mod kinds;
mod loader_sim;
mod node_data;
mod nodes;
mod options;
mod render;
mod resolve;
mod resolved;
mod state;
mod traverse;

pub use core::Compiler;
pub use driver::run_compilation;
pub use kinds::*;
pub use loader_sim::LoaderSim;
pub use node_data::*;
pub use options::*;
pub use resolved::*;
