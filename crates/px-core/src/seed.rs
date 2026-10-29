const GOLDEN: u64 = 0x9e37_79b9_7f4a_7c15;

#[inline]
#[must_use]
pub const fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// `hash_seed(seed, parts)` is `parts` folded into `seeded(seed)` with `absorb`; hot loops
/// that share a seed or leading parts compute that prefix once.
#[must_use]
pub fn hash_seed(seed: u64, parts: &[u64]) -> u64 {
    parts
        .iter()
        .fold(seeded(seed), |acc, part| absorb(acc, *part))
}

#[inline]
#[must_use]
pub const fn seeded(seed: u64) -> u64 {
    mix(seed ^ GOLDEN)
}

#[inline]
#[must_use]
pub const fn absorb(acc: u64, part: u64) -> u64 {
    mix(acc ^ part.wrapping_add(GOLDEN))
}
