//! Hole set: free power-of-two slots within a 64-bit word.
//!
//! Pattern: Strategy — encapsulates the buddy-style hole allocation policy behind a tiny API.

/// One hole per power-of-two size between 1 bit and 32 bits. Each value is the offset of the
/// hole as a multiple of its size; 0 means no hole.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct HoleSet {
    pub(super) holes: [u32; 6],
}

impl HoleSet {
    pub(super) fn try_allocate(&mut self, lg: u32) -> Option<u32> {
        if lg as usize >= 6 {
            None
        } else if self.holes[lg as usize] != 0 {
            let r = self.holes[lg as usize];
            self.holes[lg as usize] = 0;
            Some(r)
        } else if let Some(next) = self.try_allocate(lg + 1) {
            let r = next * 2;
            self.holes[lg as usize] = r + 1;
            Some(r)
        } else {
            None
        }
    }

    /// Add holes of progressively larger sizes in `[lg, limit)` starting at `offset`.
    pub(super) fn add_holes_at_end(&mut self, mut lg: u32, mut offset: u32, limit: u32) {
        debug_assert!(limit <= 6);
        while lg < limit {
            debug_assert_eq!(self.holes[lg as usize], 0);
            debug_assert_eq!(offset % 2, 1);
            self.holes[lg as usize] = offset;
            lg += 1;
            offset = offset.div_ceil(2);
        }
    }

    pub(super) fn try_expand(&mut self, old_lg: u32, old_offset: u32, factor: u32) -> bool {
        if factor == 0 {
            return true;
        }
        if old_lg == 6 {
            return false;
        }
        if self.holes[old_lg as usize] != old_offset + 1 {
            return false;
        }
        if self.try_expand(old_lg + 1, old_offset >> 1, factor - 1) {
            self.holes[old_lg as usize] = 0;
            true
        } else {
            false
        }
    }

    pub(super) fn smallest_at_least(&self, size: u32) -> Option<u32> {
        (size as usize..6)
            .find(|&i| self.holes[i] != 0)
            .map(|i| i as u32)
    }
}
