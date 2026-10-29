use crate::palette::Paint;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[must_use]
pub fn hash_cells(cells: &[Paint]) -> u64 {
    cells.iter().fold(FNV_OFFSET, |hash, cell| {
        let ramp = u16::try_from(cell.ramp.index())
            .unwrap_or(u16::MAX)
            .to_le_bytes();
        fnv(hash, &[ramp[0], ramp[1], cell.tone.index()])
    })
}

#[must_use]
pub fn hash_bytes(bytes: &[u8]) -> u64 {
    fnv(FNV_OFFSET, bytes)
}

fn fnv(hash: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(hash, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}
