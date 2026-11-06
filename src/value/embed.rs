//! Decoding of embedded serialized messages (`embed "file.bin"` for struct types).
//!
//! Pattern: Interpreter — walks the wire format (pointers, lists, far pointers) into a `Ptr` tree.

use crate::schema::{ListVal, Ptr, StructVal};

/// Words of every segment of a message.
struct Message {
    segments: Vec<Vec<u64>>,
}

/// Decode a standard-framed message and return its root struct.
pub(super) fn decode_root_struct(data: &[u8]) -> Option<StructVal> {
    if data.len() % 8 != 0 || data.len() < 8 {
        return None;
    }
    let count = u32::from_le_bytes(data[0..4].try_into().ok()?) as usize + 1;
    let header = (4 + 4 * count).div_ceil(8) * 8;
    let mut pos = header;
    let mut segments = Vec::with_capacity(count);
    for i in 0..count {
        let size = u32::from_le_bytes(data[4 + 4 * i..8 + 4 * i].try_into().ok()?) as usize;
        let bytes = data.get(pos..pos + size * 8)?;
        segments.push(
            bytes
                .chunks_exact(8)
                .map(|c| u64::from_le_bytes(c.try_into().unwrap_or_default()))
                .collect(),
        );
        pos += size * 8;
    }
    let m = Message { segments };
    match m.read_pointer(0, 0, 0)? {
        Ptr::Struct(s) => Some(s),
        _ => None,
    }
}

fn signed_offset(word: u64) -> i64 {
    i64::from((word as u32 as i32) >> 2)
}

impl Message {
    fn word(&self, seg: usize, idx: i64) -> Option<u64> {
        self.segments
            .get(seg)?
            .get(usize::try_from(idx).ok()?)
            .copied()
    }

    /// Read the pointer stored at `seg[idx]`; `depth` bounds recursion.
    fn read_pointer(&self, seg: usize, idx: i64, depth: u32) -> Option<Ptr> {
        if depth > 64 {
            return None;
        }
        let word = self.word(seg, idx)?;
        if word == 0 {
            return Some(Ptr::Null);
        }
        match word & 3 {
            0 | 1 => self.read_target(seg, idx + 1 + signed_offset(word), word, depth),
            2 => {
                let pad_seg = (word >> 32) as usize;
                let pad = i64::from((word as u32) >> 3);
                if word & 4 == 0 {
                    return self.read_pointer(pad_seg, pad, depth + 1);
                }
                let far = self.word(pad_seg, pad)?;
                let tag = self.word(pad_seg, pad + 1)?;
                self.read_target(
                    (far >> 32) as usize,
                    i64::from((far as u32) >> 3),
                    tag,
                    depth,
                )
            }
            _ => None,
        }
    }

    /// Decode the object a struct/list pointer `word` describes, starting at `seg[start]`.
    fn read_target(&self, seg: usize, start: i64, word: u64, depth: u32) -> Option<Ptr> {
        let dw = (word >> 32) & 0xffff;
        let pc = word >> 48;
        if word & 3 == 0 {
            return self.read_struct(seg, start, dw, pc, depth).map(Ptr::Struct);
        }
        let count = (word >> 35) as usize;
        match (word >> 32) & 7 {
            0 => Some(Ptr::List(ListVal::Void(count as u32))),
            1 => {
                let bits = (0..count)
                    .map(|i| Some(self.word(seg, start + (i / 64) as i64)? >> (i % 64) & 1 == 1))
                    .collect::<Option<Vec<bool>>>()?;
                Some(Ptr::List(ListVal::Bits(bits)))
            }
            code @ 2..=5 => self.read_scalars(seg, start, count, 1usize << (code - 2)),
            6 => {
                let ptrs = (0..count)
                    .map(|i| self.read_pointer(seg, start + i as i64, depth + 1))
                    .collect::<Option<Vec<_>>>()?;
                Some(Ptr::List(ListVal::Pointers(ptrs)))
            }
            _ => self.read_composite(seg, start, depth),
        }
    }

    fn read_struct(
        &self,
        seg: usize,
        start: i64,
        dw: u64,
        pc: u64,
        depth: u32,
    ) -> Option<StructVal> {
        let mut data = Vec::with_capacity(dw as usize * 8);
        for i in 0..dw as i64 {
            data.extend_from_slice(&self.word(seg, start + i)?.to_le_bytes());
        }
        let ptrs = (0..pc as i64)
            .map(|i| self.read_pointer(seg, start + dw as i64 + i, depth + 1))
            .collect::<Option<Vec<_>>>()?;
        Some(StructVal {
            data_words: dw as u16,
            pointer_count: pc as u16,
            data,
            ptrs,
        })
    }

    fn read_scalars(&self, seg: usize, start: i64, count: usize, size: usize) -> Option<Ptr> {
        let mut bytes = Vec::with_capacity(count * size);
        for i in 0..(count * size).div_ceil(8) {
            bytes.extend_from_slice(&self.word(seg, start + i as i64)?.to_le_bytes());
        }
        bytes.truncate(count * size);
        if size == 1 {
            return Some(Ptr::Data(bytes));
        }
        let values = bytes
            .chunks_exact(size)
            .map(|c| {
                let mut b = [0u8; 8];
                b[..size].copy_from_slice(c);
                u64::from_le_bytes(b)
            })
            .collect();
        Some(Ptr::List(ListVal::Bytes {
            size: size as u8,
            values,
        }))
    }

    fn read_composite(&self, seg: usize, start: i64, depth: u32) -> Option<Ptr> {
        let tag = self.word(seg, start)?;
        let (count, dw, pc) = ((tag as u32 >> 2) as usize, (tag >> 32) & 0xffff, tag >> 48);
        let stride = (dw + pc) as i64;
        let elems = (0..count as i64)
            .map(|i| self.read_struct(seg, start + 1 + i * stride, dw, pc, depth + 1))
            .collect::<Option<Vec<_>>>()?;
        Some(Ptr::List(ListVal::Structs {
            data_words: dw as u16,
            pointer_count: pc as u16,
            elems,
        }))
    }
}
