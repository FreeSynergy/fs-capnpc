//! Struct layout allocator (the `StructLayout` part of `node-translator.c++`).
//!
//! Decides data/pointer offsets for fields. The model: a struct has a data section made of
//! 64-bit words that is filled from "holes" (power-of-two sized free slots, at most one hole per
//! size), and a pointer section. Unions overlay their members in shared locations; groups
//! allocate from the locations of their parent union.
//!
//! The scope graph (`Top` <- `Union` <- `Group` <- `Union` ...) is kept in index arenas.
//!
//! Pattern: Facade — `Layout` and `Scope` are the public surface; holes and arenas stay internal.

mod allocator;
mod data;
mod expand;
mod holes;
#[cfg(test)]
mod tests;

pub use allocator::Layout;
pub use data::Scope;
