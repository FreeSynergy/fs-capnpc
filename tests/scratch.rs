mod common;
#[test]
fn dump_words() {
    let a = common::oracle_bytes(&["tests/schemas/doc.capnp"], "tests/schemas", &[]);
    let words: Vec<u64> = a[8..].chunks(8).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect();
    for (i, w) in words.iter().enumerate().skip(words.len().saturating_sub(250)) {
        let b = w.to_le_bytes();
        let txt: String = b.iter().map(|&c| if (32..127).contains(&c) { c as char } else { '.' }).collect();
        println!("W{:3} {:016x} {}", i, w, txt);
    }
}
