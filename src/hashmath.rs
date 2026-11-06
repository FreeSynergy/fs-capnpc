//! Number-theory and hash helpers for the libc++ hash table model.
//!
//! Pattern: Model — replays libc++'s prime growth and murmur2 hashing.

pub(super) fn is_prime(n: usize) -> bool {
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

pub(super) fn next_prime(mut n: usize) -> usize {
    while !is_prime(n) {
        n += 1;
    }
    n
}

/// `std::hash<uint64_t>` of libc++ on a 32-bit target (wasm32): murmur2 over the 8 key bytes.
pub(super) fn hash_key(key: u64) -> u32 {
    const M: u32 = 0x5bd1_e995;
    let mut h: u32 = 8;
    for word in [key as u32, (key >> 32) as u32] {
        let mut k = word.wrapping_mul(M);
        k ^= k >> 24;
        k = k.wrapping_mul(M);
        h = h.wrapping_mul(M) ^ k;
    }
    h ^= h >> 13;
    h = h.wrapping_mul(M);
    h ^ (h >> 15)
}
