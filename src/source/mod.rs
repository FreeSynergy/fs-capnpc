//! Source file discovery, caching and parsing (counterpart of `module-loader.c++`).
//!
//! Pattern: Strategy — [`SourceLoader`] abstracts where bytes come from (disk, memory).

mod disk;
mod loader;
mod memory;
mod module;
mod paths;
mod provider;
mod std_files;
#[cfg(test)]
mod tests;

pub use disk::DiskLoader;
pub use loader::ModuleLoader;
pub use memory::MemoryLoader;
pub use module::{Module, Resolution};
pub use paths::{Dir, eval_components};
pub use provider::SourceLoader;
pub use std_files::{STD_FILES, std_file};
