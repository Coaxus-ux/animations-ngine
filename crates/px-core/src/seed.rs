const GOLDEN: u64 = 0x9e37_79b9_7f4a_7c15;

#[must_use]
pub const fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

#[must_use]
pub fn hash_seed(seed: u64, parts: &[u64]) -> u64 {
    parts.iter().fold(mix(seed ^ GOLDEN), |acc, part| {
        mix(acc ^ part.wrapping_add(GOLDEN))
    })
}
