#[inline(never)]
pub fn fnv1a(string: &str) -> u32 {
    let mut hash = 0x811c9dc5u32;
    for c in string.chars() {
        hash = (hash ^ c.to_ascii_lowercase() as u32).wrapping_mul(0x01000193);
    }
    hash
}

#[inline(never)]
pub fn string_to_hash(string: &str) -> u32 {
    let mut hash = 0u32;
    for c in string.chars() {
        hash = (hash << 4) + c.to_ascii_lowercase() as u32;
        let temp = hash & 0xf0000000;
        if temp != 0 {
            hash ^= temp >> 24;
            hash ^= temp;
        }
    }
    hash
}
