//! Expansion of data locations for already-allocated fields (`try_expand_data`).
//!
//! Pattern: Template method — fixed locate-then-expand steps per scope kind.

use super::allocator::{ISSUE_344, Layout};
use super::data::*;

impl Layout {
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
