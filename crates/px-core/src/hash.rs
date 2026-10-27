use crate::palette::Paint;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// A stable 64-bit FNV-1a hash of indexed cells, identical on every platform.
///
/// Golden tests compare these hashes instead of RGB output (`docs/BENCHMARKS.md` §1).
#[must_use]
pub fn hash_cells(cells: &[Paint]) -> u64 {
    let mut hash = FNV_OFFSET;
    for cell in cells {
        let ramp = u16::try_from(cell.ramp.index())
            .unwrap_or(u16::MAX)
            .to_le_bytes();
        for byte in [ramp[0], ramp[1], cell.tone.index()] {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    hash
}
