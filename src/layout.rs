//! Struct layout allocator (the `StructLayout` part of `node-translator.c++`).
//!
//! Decides data/pointer offsets for fields. The model: a struct has a data section made of
//! 64-bit words that is filled from "holes" (power-of-two sized free slots, at most one hole per
//! size), and a pointer section. Unions overlay their members in shared locations; groups
//! allocate from the locations of their parent union.
//!
//! The scope graph (`Top` <- `Union` <- `Group` <- `Union` ...) is kept in index arenas.

/// One hole per power-of-two size between 1 bit and 32 bits. Each value is the offset of the
/// hole as a multiple of its size; 0 means no hole.
#[derive(Clone, Copy, Debug, Default)]
struct HoleSet {
    holes: [u32; 6],
}

impl HoleSet {
    fn try_allocate(&mut self, lg: u32) -> Option<u32> {
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
    fn add_holes_at_end(&mut self, mut lg: u32, mut offset: u32, limit: u32) {
        debug_assert!(limit <= 6);
        while lg < limit {
            debug_assert_eq!(self.holes[lg as usize], 0);
            debug_assert_eq!(offset % 2, 1);
            self.holes[lg as usize] = offset;
            lg += 1;
            offset = offset.div_ceil(2);
        }
    }

    fn try_expand(&mut self, old_lg: u32, old_offset: u32, factor: u32) -> bool {
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

    fn smallest_at_least(&self, size: u32) -> Option<u32> {
        (size as usize..6).find(|&i| self.holes[i] != 0).map(|i| i as u32)
    }
}

#[derive(Clone, Copy, Debug)]
struct DataLocation {
    lg_size: u32,
    offset: u32,
}

/// Reference to a scope in which fields are added.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Top,
    Group(usize),
}

#[derive(Clone, Copy, Debug, Default)]
struct DataLocationUsage {
    is_used: bool,
    lg_size_used: u32,
    holes: HoleSet,
}

#[derive(Debug)]
struct UnionData {
    parent: Scope,
    group_count: u32,
    discriminant_offset: Option<u32>,
    data_locations: Vec<DataLocation>,
    pointer_locations: Vec<u32>,
}

#[derive(Debug)]
struct GroupData {
    parent: usize, // union id
    usage: Vec<DataLocationUsage>,
    pointer_usage: usize,
    has_members: bool,
}

/// The layout state for one struct.
#[derive(Debug, Default)]
pub struct Layout {
    pub data_word_count: u32,
    pub pointer_count: u32,
    top_holes: HoleSet,
    unions: Vec<UnionData>,
    groups: Vec<GroupData>,
    /// Set when the layout hit the "issue 344" situation (schema compiled incorrectly by capnp <= 0.5).
    pub error: Option<String>,
}

const ISSUE_344: &str = "Bad news: Cap'n Proto 0.5.x and previous contained a bug which would cause this schema to be compiled incorrectly. Please see: https://github.com/capnproto/capnproto/issues/344";

impl Layout {
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
        self.groups.push(GroupData { parent, usage: vec![], pointer_usage: 0, has_members: false });
        Scope::Group(self.groups.len() - 1)
    }

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

    fn group_add_member(&mut self, g: usize) {
        if !self.groups[g].has_members {
            self.groups[g].has_members = true;
            let u = self.groups[g].parent;
            self.unions[u].group_count += 1;
            if self.unions[u].group_count == 2 {
                self.add_discriminant(u);
            }
        }
    }

    fn smallest_hole_at_least(usage: &DataLocationUsage, location: &DataLocation, lg: u32) -> Option<u32> {
        if !usage.is_used {
            if lg <= location.lg_size { Some(location.lg_size) } else { None }
        } else if lg >= usage.lg_size_used {
            if lg < location.lg_size { Some(lg) } else { None }
        } else if let Some(r) = usage.holes.smallest_at_least(lg) {
            Some(r)
        } else if usage.lg_size_used < location.lg_size {
            Some(usage.lg_size_used)
        } else {
            None
        }
    }

    fn allocate_from_hole(usage: &mut DataLocationUsage, location: &DataLocation, lg: u32) -> u32 {
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
            usage.holes.add_holes_at_end(lg, result + 1, usage.lg_size_used);
            usage.lg_size_used += 1;
        }
        (location.offset << (location.lg_size - lg)) + result
    }

    /// `location.tryExpandTo(union, newLg)`.
    fn location_try_expand_to(&mut self, union: usize, g_loc: usize, new_lg: u32) -> bool {
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

    fn usage_try_expand_usage(
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

    fn try_allocate_by_expanding(&mut self, g: usize, loc_idx: usize, lg: u32) -> Option<u32> {
        let union = self.groups[g].parent;
        let mut usage = self.groups[g].usage[loc_idx];
        let result;
        if !usage.is_used {
            if self.location_try_expand_to(union, loc_idx, lg) {
                usage.is_used = true;
                usage.lg_size_used = lg;
                let loc = self.unions[union].data_locations[loc_idx];
                result = Some(loc.offset << (loc.lg_size - lg));
            } else {
                result = None;
            }
        } else {
            let new_size = usage.lg_size_used.max(lg) + 1;
            if self.usage_try_expand_usage(union, loc_idx, &mut usage, new_size, true) {
                let r = usage.holes.try_allocate(lg).expect("hole must exist after expansion");
                let loc = self.unions[union].data_locations[loc_idx];
                result = Some((loc.offset << (loc.lg_size - lg)) + r);
            } else {
                result = None;
            }
        }
        self.groups[g].usage[loc_idx] = usage;
        result
    }

    fn group_add_data(&mut self, g: usize, lg: u32) -> u32 {
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
            if let Some(hole) = Self::smallest_hole_at_least(&usage, &loc, lg) {
                if hole < best_size {
                    best_size = hole;
                    best_loc = Some(i);
                }
            }
        }
        if let Some(b) = best_loc {
            let loc = self.unions[union].data_locations[b];
            let mut usage = self.groups[g].usage[b];
            let r = Self::allocate_from_hole(&mut usage, &loc, lg);
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
        self.unions[union].data_locations.push(DataLocation { lg_size: lg, offset });
        self.groups[g].usage.push(DataLocationUsage { is_used: true, lg_size_used: lg, holes: HoleSet::default() });
        offset
    }

    fn usage_try_expand(
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

    pub fn try_expand_data(&mut self, scope: Scope, old_lg: u32, old_offset: u32, factor: u32) -> bool {
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
                    if loc.lg_size >= old_lg && (old_offset >> (loc.lg_size - old_lg)) == loc.offset {
                        let local_old_offset = old_offset - (loc.offset << (loc.lg_size - old_lg));
                        let mut usage = self.groups[g].usage[i];
                        let result =
                            self.usage_try_expand(union, i, &mut usage, old_lg, local_old_offset, factor);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_level_packing() {
        let mut l = Layout::new();
        // Bool, Int8, Int16, Int32, Bool -> all fit in one word.
        let a = l.add_data(Scope::Top, 0);
        let b = l.add_data(Scope::Top, 3);
        let c = l.add_data(Scope::Top, 4);
        let d = l.add_data(Scope::Top, 5);
        let e = l.add_data(Scope::Top, 0);
        assert_eq!((a, b, c, d, e), (0, 1, 1, 1, 1));
        assert_eq!(l.data_word_count, 1);
        // A 64-bit value needs a new word.
        assert_eq!(l.add_data(Scope::Top, 6), 1);
        assert_eq!(l.data_word_count, 2);
    }

    #[test]
    fn pointers_sequential() {
        let mut l = Layout::new();
        assert_eq!(l.add_pointer(Scope::Top), 0);
        assert_eq!(l.add_pointer(Scope::Top), 1);
        assert_eq!(l.pointer_count, 2);
    }

    #[test]
    fn union_shares_and_adds_discriminant() {
        let mut l = Layout::new();
        let u = l.new_union(Scope::Top);
        let g1 = l.new_group(u);
        let g2 = l.new_group(u);
        // member 1: Int32 at offset 0 (32-bit slot 0), member 2: Text pointer; discriminant added on 2nd.
        let a = l.add_data(g1, 5);
        let p = l.add_pointer(g2);
        assert_eq!(a, 0);
        assert_eq!(p, 0);
        assert!(l.discriminant_offset(u).is_some());
        assert_eq!(l.discriminant_offset(u), Some(2));
        assert_eq!(l.data_word_count, 1);
        assert_eq!(l.pointer_count, 1);
    }

    #[test]
    fn union_members_overlap_data() {
        let mut l = Layout::new();
        let u = l.new_union(Scope::Top);
        let g1 = l.new_group(u);
        let g2 = l.new_group(u);
        let g3 = l.new_group(u);
        let a = l.add_data(g1, 5); // Int32
        let b = l.add_data(g2, 6); // Int64: discriminant took the neighbour, so a new word
        let c = l.add_data(g3, 3); // Int8 overlaps the Int32 of the first member
        assert_eq!(a, 0);
        assert_eq!(b, 1);
        assert_eq!(c, 0);
    }
}
