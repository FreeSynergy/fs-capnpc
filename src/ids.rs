//! Type id generation (counterpart of `type-id.c++`).
//!
//! Pattern: Strategy — id derivation is a pure function from a parent id and a name.
//!
//! IDs are the first 8 bytes of an MD5 digest,
//! read big-endian, with the top bit set.

fn md5(data: &[u8]) -> [u8; 16] {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    let k: Vec<u32> = (0..64)
        .map(|i| ((f64::from(i) + 1.0).sin().abs() * 4294967296.0) as u32)
        .collect();
    let (mut a0, mut b0, mut c0, mut d0) =
        (0x67452301u32, 0xefcdab89u32, 0x98badcfeu32, 0x10325476u32);

    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64).wrapping_mul(8)).to_le_bytes());

    for chunk in msg.chunks(64) {
        let m: Vec<u32> = chunk
            .chunks(4)
            .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
            .collect();
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (mut f, g);
            match i / 16 {
                0 => {
                    f = (b & c) | (!b & d);
                    g = i;
                }
                1 => {
                    f = (d & b) | (!d & c);
                    g = (5 * i + 1) % 16;
                }
                2 => {
                    f = b ^ c ^ d;
                    g = (3 * i + 5) % 16;
                }
                _ => {
                    f = c ^ (b | !d);
                    g = (7 * i) % 16;
                }
            }
            f = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f.rotate_left(S[i]));
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }
    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&a0.to_le_bytes());
    out[4..8].copy_from_slice(&b0.to_le_bytes());
    out[8..12].copy_from_slice(&c0.to_le_bytes());
    out[12..16].copy_from_slice(&d0.to_le_bytes());
    out
}

fn first8(digest: &[u8; 16]) -> u64 {
    let mut r = 0u64;
    for &b in &digest[..8] {
        r = (r << 8) | u64::from(b);
    }
    r | (1u64 << 63)
}

/// Id of a child declaration `child_name` of the node with id `parent_id`.
pub fn generate_child_id(parent_id: u64, child_name: &str) -> u64 {
    let mut bytes = parent_id.to_le_bytes().to_vec();
    bytes.extend_from_slice(child_name.as_bytes());
    first8(&md5(&bytes))
}

/// Id of the `group_index`-th field group of a struct.
pub fn generate_group_id(parent_id: u64, group_index: u16) -> u64 {
    let mut bytes = parent_id.to_le_bytes().to_vec();
    bytes.extend_from_slice(&group_index.to_le_bytes());
    first8(&md5(&bytes))
}

/// Id of the synthetic params/results struct of a method.
pub fn generate_method_params_id(parent_id: u64, method_ordinal: u16, is_results: bool) -> u64 {
    let mut bytes = parent_id.to_le_bytes().to_vec();
    bytes.extend_from_slice(&method_ordinal.to_le_bytes());
    bytes.push(u8::from(is_results));
    first8(&md5(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(d: &[u8]) -> String {
        d.iter().fold(String::new(), |mut s, b| {
            use std::fmt::Write;
            let _ = write!(s, "{b:02x}");
            s
        })
    }

    #[test]
    fn md5_vectors() {
        assert_eq!(hex(&md5(b"")), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(hex(&md5(b"abc")), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(
            hex(&md5(
                b"12345678901234567890123456789012345678901234567890123456789012345678901234567890"
            )),
            "57edf4a22be3c955ac49da2e2107b67a"
        );
    }

    #[test]
    fn ids_match_official_compiler() {
        // From type-id-test.c++.
        assert_eq!(
            generate_child_id(0xa93fc509624c72d9, "Node"),
            0xe682ab4cf923a417
        );
        assert_eq!(
            generate_child_id(0xe682ab4cf923a417, "NestedNode"),
            0xdebf55bbfa0fc242
        );
        assert_eq!(generate_group_id(0xe682ab4cf923a417, 7), 0x9ea0b19b37fb4435);
        assert_eq!(
            generate_method_params_id(0x88eb12a0e0af92b2, 0, false),
            0xb874edc0d559b391
        );
        assert_eq!(
            generate_method_params_id(0x88eb12a0e0af92b2, 0, true),
            0xb04fcaddab714ba4
        );
        assert_eq!(
            generate_method_params_id(0x88eb12a0e0af92b2, 1, false),
            0xd044893357b42568
        );
        assert_eq!(
            generate_method_params_id(0x88eb12a0e0af92b2, 1, true),
            0x9bf141df4247d52f
        );
    }
}
