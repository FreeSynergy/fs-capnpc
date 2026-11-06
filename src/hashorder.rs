//! Iteration order of the official compiler's `sourceInfoById` map.
//!
//! Pattern: Model — replays the C++ standard library's hash table so output order matches exactly.
//!
//! The official compiler keeps source info in a `std::unordered_map<uint64_t, ...>` (libc++ when
//! built for WASI) and emits it in iteration order. To produce an identical request we replay the
//! insertions into a model of libc++'s `__hash_table` (singly linked list + bucket array, identity
//! hash, max load factor 1.0, bucket counts growing through 2, 5, 11, 23, ... primes).

use crate::schema::SourceInfo;

fn is_prime(n: usize) -> bool {
    if n < 2 {
        return false;
    }
    let mut i = 2;
    while i * i <= n {
        if n.is_multiple_of(i) {
            return false;
        }
        i += 1;
    }
    true
}

fn next_prime(mut n: usize) -> usize {
    while !is_prime(n) {
        n += 1;
    }
    n
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Before {
    Head,
    Node(usize),
}

struct Table {
    keys: Vec<u64>,
    next: Vec<Option<usize>>,
    head: Option<usize>,
    buckets: Vec<Option<Before>>,
}

impl Table {
    fn new() -> Self {
        Table {
            keys: vec![],
            next: vec![],
            head: None,
            buckets: vec![],
        }
    }

    fn constrain(h: u64, bc: usize) -> usize {
        if bc.is_power_of_two() {
            (h as usize) & (bc - 1)
        } else {
            (h % bc as u64) as usize
        }
    }

    fn next_of(&self, b: Before) -> Option<usize> {
        match b {
            Before::Head => self.head,
            Before::Node(i) => self.next[i],
        }
    }

    fn set_next(&mut self, b: Before, v: Option<usize>) {
        match b {
            Before::Head => self.head = v,
            Before::Node(i) => self.next[i] = v,
        }
    }

    fn do_rehash(&mut self, nbc: usize) {
        self.buckets = vec![None; nbc];
        let mut pp = Before::Head;
        let Some(first) = self.head else { return };
        let mut chash = Self::constrain(self.keys[first], nbc);
        self.buckets[chash] = Some(pp);
        pp = Before::Node(first);
        while let Some(cp) = self.next_of(pp) {
            let nhash = Self::constrain(self.keys[cp], nbc);
            if nhash == chash {
                pp = Before::Node(cp);
            } else if self.buckets[nhash].is_none() {
                self.buckets[nhash] = Some(pp);
                pp = Before::Node(cp);
                chash = nhash;
            } else {
                let target = self.buckets[nhash].unwrap();
                let after_cp = self.next[cp];
                self.set_next(pp, after_cp);
                let tn = self.next_of(target);
                self.next[cp] = tn;
                self.set_next(target, Some(cp));
            }
        }
    }

    fn insert(&mut self, key: u64) {
        let bc = self.buckets.len();
        let size = self.keys.len();
        if bc == 0 || size + 1 > bc {
            let is_pow2 = bc > 2 && bc.is_power_of_two();
            let want = (2 * bc + usize::from(!is_pow2)).max(size + 1);
            let mut n = want;
            if n == 1 {
                n = 2;
            } else if n & (n - 1) != 0 {
                n = next_prime(n);
            }
            if n > bc {
                self.do_rehash(n);
            }
        }
        let bc = self.buckets.len();
        let chash = Self::constrain(key, bc);
        let idx = self.keys.len();
        self.keys.push(key);
        self.next.push(None);
        match self.buckets[chash] {
            None => {
                self.next[idx] = self.head;
                self.head = Some(idx);
                self.buckets[chash] = Some(Before::Head);
                if let Some(n) = self.next[idx] {
                    let nh = Self::constrain(self.keys[n], bc);
                    self.buckets[nh] = Some(Before::Node(idx));
                }
            }
            Some(pn) => {
                let after = self.next_of(pn);
                self.next[idx] = after;
                self.set_next(pn, Some(idx));
            }
        }
    }

    fn iterate(&self) -> Vec<usize> {
        let mut out = vec![];
        let mut cur = self.head;
        while let Some(i) = cur {
            out.push(i);
            cur = self.next[i];
        }
        out
    }
}

/// Order `items` (given in insertion order, unique ids) the way the official compiler emits them.
pub fn order_source_info(items: Vec<SourceInfo>) -> Vec<SourceInfo> {
    let mut t = Table::new();
    for it in &items {
        t.insert(it.id);
    }
    let order = t.iterate();
    let mut slots: Vec<Option<SourceInfo>> = items.into_iter().map(Some).collect();
    order
        .into_iter()
        .map(|i| slots[i].take().unwrap())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(keys: &[u64]) -> Vec<u64> {
        let items = keys
            .iter()
            .map(|&id| SourceInfo {
                id,
                ..Default::default()
            })
            .collect();
        order_source_info(items).into_iter().map(|s| s.id).collect()
    }

    #[test]
    fn small_tables() {
        // bucket count 2: even keys all in bucket 0, inserted at the front of the chain.
        assert_eq!(order(&[4, 6]), vec![6, 4]);
        assert_eq!(order(&[1, 2]), vec![2, 1]);
    }

    #[test]
    fn matches_official_sample() {
        // Insertion order file, Person, Person.addr produced Person, addr, file in capnp 1.4.0.
        let f = 15832876314001345334u64;
        let p = 17103488608928808786u64;
        let a = 18004147750951456554u64;
        assert_eq!(order(&[f, p, a]), vec![p, a, f]);
    }
}
