//! Flat encoding of pointer value trees into a one-segment message.
//!
//! Pattern: Builder — assembles raw words and pointers into an encoded segment.

use crate::schema::*;
use capnp::message::ReaderOptions;

pub(super) struct Enc {
    words: Vec<u64>,
}

impl Enc {
    fn alloc(&mut self, n: usize) -> usize {
        let p = self.words.len();
        self.words.resize(p + n, 0);
        p
    }

    fn put_bytes(&mut self, pos: usize, bytes: &[u8]) {
        for (i, chunk) in bytes.chunks(8).enumerate() {
            let mut w = [0u8; 8];
            w[..chunk.len()].copy_from_slice(chunk);
            self.words[pos + i] = u64::from_le_bytes(w);
        }
    }

    fn ptr_word(slot: usize, target: usize, kind: u32, hi: u32) -> u64 {
        let off = (target as i64 - (slot as i64 + 1)) as i32;
        let lo = ((off as u32) << 2) | kind;
        (lo as u64) | ((hi as u64) << 32)
    }

    fn encode_struct_body(&mut self, sv: &StructVal) -> usize {
        let dw = sv.data_words as usize;
        let pc = sv.pointer_count as usize;
        let pos = self.alloc(dw + pc);
        self.put_bytes(pos, &sv.data);
        for (i, p) in sv.ptrs.iter().enumerate() {
            self.encode(p, pos + dw + i);
        }
        pos
    }

    fn encode(&mut self, p: &Ptr, slot: usize) {
        match p {
            Ptr::Null => {}
            Ptr::Struct(sv) => {
                let pos = self.encode_struct_body(sv);
                self.words[slot] = Self::ptr_word(
                    slot,
                    pos,
                    0,
                    sv.data_words as u32 | ((sv.pointer_count as u32) << 16),
                );
            }
            Ptr::Text(t) => {
                let mut b = t.clone();
                b.push(0);
                let pos = self.alloc(b.len().div_ceil(8));
                self.put_bytes(pos, &b);
                self.words[slot] = Self::ptr_word(slot, pos, 1, 2 | ((b.len() as u32) << 3));
            }
            Ptr::Data(d) => {
                let pos = self.alloc(d.len().div_ceil(8));
                self.put_bytes(pos, d);
                self.words[slot] = Self::ptr_word(slot, pos, 1, 2 | ((d.len() as u32) << 3));
            }
            Ptr::List(l) => match l {
                ListVal::Void(n) => {
                    let pos = self.words.len();
                    self.words[slot] = Self::ptr_word(slot, pos, 1, *n << 3);
                }
                ListVal::Bits(bits) => {
                    let pos = self.alloc(bits.len().div_ceil(64));
                    for (i, b) in bits.iter().enumerate() {
                        if *b {
                            self.words[pos + i / 64] |= 1u64 << (i % 64);
                        }
                    }
                    self.words[slot] = Self::ptr_word(slot, pos, 1, 1 | ((bits.len() as u32) << 3));
                }
                ListVal::Bytes { size, values } => {
                    let size = *size as usize;
                    let total = values.len() * size;
                    let pos = self.alloc(total.div_ceil(8));
                    let mut bytes = vec![];
                    for v in values {
                        bytes.extend_from_slice(&v.to_le_bytes()[..size]);
                    }
                    self.put_bytes(pos, &bytes);
                    let code = match size {
                        1 => 2,
                        2 => 3,
                        4 => 4,
                        _ => 5,
                    };
                    self.words[slot] =
                        Self::ptr_word(slot, pos, 1, code | ((values.len() as u32) << 3));
                }
                ListVal::Pointers(ps) => {
                    let pos = self.alloc(ps.len());
                    for (i, p) in ps.iter().enumerate() {
                        self.encode(p, pos + i);
                    }
                    self.words[slot] = Self::ptr_word(slot, pos, 1, 6 | ((ps.len() as u32) << 3));
                }
                ListVal::Structs {
                    data_words,
                    pointer_count,
                    elems,
                } => {
                    let per = *data_words as usize + *pointer_count as usize;
                    let pos = self.alloc(1 + elems.len() * per);
                    self.words[pos] = Self::ptr_word(pos, elems.len(), 0, 0) & 0xffff_ffff
                        | (((*data_words as u64) | ((*pointer_count as u64) << 16)) << 32);
                    // tag word: element count in the offset field
                    self.words[pos] = ((elems.len() as u64) << 2)
                        | (((*data_words as u64) | ((*pointer_count as u64) << 16)) << 32);
                    for (i, e) in elems.iter().enumerate() {
                        let base = pos + 1 + i * per;
                        self.put_bytes(base, &e.data);
                        for (j, p) in e.ptrs.iter().enumerate() {
                            self.encode(p, base + *data_words as usize + j);
                        }
                    }
                    self.words[slot] =
                        Self::ptr_word(slot, pos, 1, 7 | (((elems.len() * per) as u32) << 3));
                }
            },
        }
    }
}

/// Build a one-segment serialized message whose root is `ptr`.
pub(super) fn flat_message(ptr: &Ptr) -> Vec<u8> {
    let mut e = Enc { words: vec![0] };
    e.encode(ptr, 0);
    let mut out = Vec::new();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(e.words.len() as u32).to_le_bytes());
    for w in &e.words {
        out.extend_from_slice(&w.to_le_bytes());
    }
    out
}

pub(super) fn set_any_pointer(mut target: capnp::any_pointer::Builder<'_>, ptr: &Ptr) {
    let bytes = flat_message(ptr);
    let reader = capnp::serialize::read_message(&mut &bytes[..], ReaderOptions::new())
        .expect("flat message");
    let root: capnp::any_pointer::Reader = reader.get_root().expect("root");
    target
        .set_as::<capnp::any_pointer::Owned>(root)
        .expect("copy value");
}
