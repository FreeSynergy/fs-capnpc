//! Allocator entry points: the `Layout` state plus union/group/pointer/data allocation.
//!
//! Pattern: Facade — `Layout` is the single entry point hiding holes, unions and groups.

use super::data::*;
use super::holes::HoleSet;

/// The layout state for one struct.
#[derive(Debug, Default)]
pub struct Layout {
    /// The `data_word_count` field.
    pub data_word_count: u32,
    /// The `pointer_count` field.
    pub pointer_count: u32,
    pub(super) top_holes: HoleSet,
    pub(super) unions: Vec<UnionData>,
    pub(super) groups: Vec<GroupData>,
    /// Set when the layout hit the "issue 344" situation (schema compiled incorrectly by capnp <= 0.5).
    pub error: Option<String>,
}

pub(super) const ISSUE_344: &str = "Bad news: Cap'n Proto 0.5.x and previous contained a bug which would cause this schema to be compiled incorrectly. Please see: https://github.com/capnproto/capnproto/issues/344";

impl Layout {
    /// Operation `new`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a union whose parent is `parent`. Returns its id.
    pub fn new_union(&mut self, parent: Scope) -> usize {
        self.unions.push(UnionData {
            parent,
            group_count: 0,
            discriminant_offset: None,
            data_locations: vec![],
            pointer_locations: vec![],
        });
        self.unions.len() - 1
    }

    /// Create a group (member scope) inside union `parent`.
    pub fn new_group(&mut self, parent: usize) -> Scope {
        self.groups.push(GroupData {
            parent,
            usage: vec![],
            pointer_usage: 0,
            has_members: false,
        });
        Scope::Group(self.groups.len() - 1)
    }

    /// Operation `discriminant_offset`.
    pub fn discriminant_offset(&self, union: usize) -> Option<u32> {
        self.unions[union].discriminant_offset
    }

    /// Add the union's discriminant if it does not exist yet. Returns true if it was newly added.
    pub fn add_discriminant(&mut self, union: usize) -> bool {
        if self.unions[union].discriminant_offset.is_none() {
            let parent = self.unions[union].parent;
            let off = self.add_data(parent, 4);
            self.unions[union].discriminant_offset = Some(off);
            true
        } else {
            false
        }
    }

    /// Operation `add_void`.
    pub fn add_void(&mut self, scope: Scope) {
        match scope {
            Scope::Top => {}
            Scope::Group(g) => {
                self.group_add_member(g);
                let u = self.groups[g].parent;
                let p = self.unions[u].parent;
                self.add_void(p);
            }
        }
    }

    /// Operation `add_pointer`.
    pub fn add_pointer(&mut self, scope: Scope) -> u32 {
        match scope {
            Scope::Top => {
                self.pointer_count += 1;
                self.pointer_count - 1
            }
            Scope::Group(g) => {
                self.group_add_member(g);
                let u = self.groups[g].parent;
                let used = self.groups[g].pointer_usage;
                if used < self.unions[u].pointer_locations.len() {
                    self.groups[g].pointer_usage += 1;
                    self.unions[u].pointer_locations[used]
                } else {
                    self.groups[g].pointer_usage += 1;
                    let parent = self.unions[u].parent;
                    let p = self.add_pointer(parent);
                    self.unions[u].pointer_locations.push(p);
                    p
                }
            }
        }
    }

    /// Operation `add_data`.
    pub fn add_data(&mut self, scope: Scope, lg: u32) -> u32 {
        match scope {
            Scope::Top => {
                if let Some(h) = self.top_holes.try_allocate(lg) {
                    h
                } else {
                    let offset = self.data_word_count << (6 - lg);
                    self.data_word_count += 1;
                    self.top_holes.add_holes_at_end(lg, offset + 1, 6);
                    offset
                }
            }
            Scope::Group(g) => self.group_add_data(g, lg),
        }
    }

    pub(super) fn group_add_member(&mut self, g: usize) {
        if !self.groups[g].has_members {
            self.groups[g].has_members = true;
            let u = self.groups[g].parent;
            self.unions[u].group_count += 1;
            if self.unions[u].group_count == 2 {
                self.add_discriminant(u);
            }
        }
    }
}
