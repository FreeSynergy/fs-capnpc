//! Unit tests for the layout allocator.
//!
//! Pattern: Facade — exercises the allocator only through its public API.

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
