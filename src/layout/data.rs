//! Plain data records for the layout arenas (locations, usage, unions, groups).
//!
//! Pattern: Registry — index-based arenas of small records referenced by id.

use super::holes::HoleSet;

#[derive(Clone, Copy, Debug)]
pub(super) struct DataLocation {
    pub(super) lg_size: u32,
    pub(super) offset: u32,
}

/// Reference to a scope in which fields are added.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// The struct body itself.
    Top,
    /// The group with the given arena index.
    Group(usize),
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct DataLocationUsage {
    pub(super) is_used: bool,
    pub(super) lg_size_used: u32,
    pub(super) holes: HoleSet,
}

#[derive(Debug)]
pub(super) struct UnionData {
    pub(super) parent: Scope,
    pub(super) group_count: u32,
    pub(super) discriminant_offset: Option<u32>,
    pub(super) data_locations: Vec<DataLocation>,
    pub(super) pointer_locations: Vec<u32>,
}

#[derive(Debug)]
pub(super) struct GroupData {
    pub(super) parent: usize, // union id
    pub(super) usage: Vec<DataLocationUsage>,
    pub(super) pointer_usage: usize,
    pub(super) has_members: bool,
}
