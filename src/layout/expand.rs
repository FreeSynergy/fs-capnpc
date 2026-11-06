//! Expansion logic of the layout allocator: finding holes and growing union locations.
//!
//! Pattern: Template method — fixed allocate-or-expand steps over group/union data.

use super::allocator::{ISSUE_344, Layout};
use super::data::*;
use super::holes::HoleSet;

impl Layout {
    pub(super) fn smallest_hole_at_least(
        usage: &DataLocationUsage,
        location: DataLocation,
        lg: u32,
    ) -> Option<u32> {
        if !usage.is_used {
            if lg <= location.lg_size {
                Some(location.lg_size)
            } else {
                None
            }
        } else if lg >= usage.lg_size_used {
            if lg < location.lg_size {
                Some(lg)
            } else {
                None
            }
        } else if let Some(r) = usage.holes.smallest_at_least(lg) {
            Some(r)
        } else if usage.lg_size_used < location.lg_size {
            Some(usage.lg_size_used)
        } else {
            None
        }
    }

    pub(super) fn allocate_from_hole(
        usage: &mut DataLocationUsage,
        location: DataLocation,
        lg: u32,
    ) -> u32 {
        let result;
        if !usage.is_used {
            result = 0;
            usage.is_used = true;
            usage.lg_size_used = lg;
        } else if lg >= usage.lg_size_used {
            usage.holes.add_holes_at_end(usage.lg_size_used, 1, lg);
            usage.lg_size_used = lg + 1;
            result = 1;
        } else if let Some(h) = usage.holes.try_allocate(lg) {
            result = h;
        } else {
            result = 1 << (usage.lg_size_used - lg);
            usage
                .holes
                .add_holes_at_end(lg, result + 1, usage.lg_size_used);
            usage.lg_size_used += 1;
        }
        (location.offset << (location.lg_size - lg)) + result
    }

    /// `location.tryExpandTo(union, newLg)`.
    pub(super) fn location_try_expand_to(
        &mut self,
        union: usize,
        g_loc: usize,
        new_lg: u32,
    ) -> bool {
        let loc = self.unions[union].data_locations[g_loc];
        if new_lg <= loc.lg_size {
            return true;
        }
        let parent = self.unions[union].parent;
        if self.try_expand_data(parent, loc.lg_size, loc.offset, new_lg - loc.lg_size) {
            let l = &mut self.unions[union].data_locations[g_loc];
            l.offset >>= new_lg - loc.lg_size;
            l.lg_size = new_lg;
            true
        } else {
            false
        }
    }

    pub(super) fn usage_try_expand_usage(
        &mut self,
        union: usize,
        loc_idx: usize,
        usage: &mut DataLocationUsage,
        desired: u32,
        new_holes: bool,
    ) -> bool {
        if desired > self.unions[union].data_locations[loc_idx].lg_size
            && !self.location_try_expand_to(union, loc_idx, desired)
        {
            return false;
        }
        if new_holes {
            usage.holes.add_holes_at_end(usage.lg_size_used, 1, desired);
        } else {
            self.error = Some(ISSUE_344.into());
        }
        usage.lg_size_used = desired;
        true
    }

    pub(super) fn try_allocate_by_expanding(
        &mut self,
        g: usize,
        loc_idx: usize,
        lg: u32,
    ) -> Option<u32> {
        let union = self.groups[g].parent;
        let mut usage = self.groups[g].usage[loc_idx];
        let result;
        if usage.is_used {
            let new_size = usage.lg_size_used.max(lg) + 1;
            if self.usage_try_expand_usage(union, loc_idx, &mut usage, new_size, true) {
                let r = usage
                    .holes
                    .try_allocate(lg)
                    .expect("hole must exist after expansion");
                let loc = self.unions[union].data_locations[loc_idx];
                result = Some((loc.offset << (loc.lg_size - lg)) + r);
            } else {
                result = None;
            }
        } else if self.location_try_expand_to(union, loc_idx, lg) {
            usage.is_used = true;
            usage.lg_size_used = lg;
            let loc = self.unions[union].data_locations[loc_idx];
            result = Some(loc.offset << (loc.lg_size - lg));
        } else {
            result = None;
        }
        self.groups[g].usage[loc_idx] = usage;
        result
    }

    pub(super) fn group_add_data(&mut self, g: usize, lg: u32) -> u32 {
        self.group_add_member(g);
        let union = self.groups[g].parent;

        let mut best_size = u32::MAX;
        let mut best_loc: Option<usize> = None;
        let n = self.unions[union].data_locations.len();
        for i in 0..n {
            if self.groups[g].usage.len() == i {
                self.groups[g].usage.push(DataLocationUsage::default());
            }
            let usage = self.groups[g].usage[i];
            let loc = self.unions[union].data_locations[i];
            if let Some(hole) = Self::smallest_hole_at_least(&usage, loc, lg)
                && hole < best_size
            {
                best_size = hole;
                best_loc = Some(i);
            }
        }
        if let Some(b) = best_loc {
            let loc = self.unions[union].data_locations[b];
            let mut usage = self.groups[g].usage[b];
            let r = Self::allocate_from_hole(&mut usage, loc, lg);
            self.groups[g].usage[b] = usage;
            return r;
        }

        for i in 0..n {
            if let Some(r) = self.try_allocate_by_expanding(g, i, lg) {
                return r;
            }
        }

        let parent = self.unions[union].parent;
        let offset = self.add_data(parent, lg);
        self.unions[union].data_locations.push(DataLocation {
            lg_size: lg,
            offset,
        });
        self.groups[g].usage.push(DataLocationUsage {
            is_used: true,
            lg_size_used: lg,
            holes: HoleSet::default(),
        });
        offset
    }

    pub(super) fn usage_try_expand(
        &mut self,
        union: usize,
        loc_idx: usize,
        usage: &mut DataLocationUsage,
        old_lg: u32,
        old_offset: u32,
        factor: u32,
    ) -> bool {
        if old_offset == 0 && usage.lg_size_used == old_lg {
            self.usage_try_expand_usage(union, loc_idx, usage, old_lg + factor, false)
        } else {
            usage.holes.try_expand(old_lg, old_offset, factor)
        }
    }

    /// Operation `try_expand_data`.
    pub fn try_expand_data(
        &mut self,
        scope: Scope,
        old_lg: u32,
        old_offset: u32,
        factor: u32,
    ) -> bool {
        match scope {
            Scope::Top => self.top_holes.try_expand(old_lg, old_offset, factor),
            Scope::Group(g) => {
                let mut must_fail = false;
                if old_lg + factor > 6 || (old_offset & ((1u32 << factor) - 1)) != 0 {
                    must_fail = true;
                }
                let union = self.groups[g].parent;
                let n = self.groups[g].usage.len();
                for i in 0..n {
                    let loc = self.unions[union].data_locations[i];
                    if loc.lg_size >= old_lg && (old_offset >> (loc.lg_size - old_lg)) == loc.offset
                    {
                        let local_old_offset = old_offset - (loc.offset << (loc.lg_size - old_lg));
                        let mut usage = self.groups[g].usage[i];
                        let result = self.usage_try_expand(
                            union,
                            i,
                            &mut usage,
                            old_lg,
                            local_old_offset,
                            factor,
                        );
                        self.groups[g].usage[i] = usage;
                        if must_fail && result {
                            self.error = Some(ISSUE_344.into());
                        }
                        return result;
                    }
                }
                self.error = Some("Tried to expand field that was never allocated.".into());
                false
            }
        }
    }
}
