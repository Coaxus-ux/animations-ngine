use px_core::Rgb;

use crate::error::ExportError;

pub(crate) const GIF_MAX_COLORS: usize = 256;

const BITS: u32 = 10;
/// Four times the 256 colors a table holds, so probes stay short.
const CAPACITY: usize = 1 << BITS;
const OCCUPIED: u32 = 1 << 24;

/// A frame's color table in first-seen order, indexed by an open-addressing map from color
/// to slot: hashing every pixel through a `HashMap` was a visible share of export time.
pub(crate) struct ColorSlots {
    keys: [u32; CAPACITY],
    slots: [u8; CAPACITY],
    table: Vec<u8>,
}

impl Default for ColorSlots {
    fn default() -> Self {
        Self {
            keys: [0; CAPACITY],
            slots: [0; CAPACITY],
            table: Vec::with_capacity(GIF_MAX_COLORS * 3),
        }
    }
}

impl ColorSlots {
    pub(crate) fn clear(&mut self) {
        self.keys.fill(0);
        self.table.clear();
    }

    pub(crate) fn table(&self) -> &[u8] {
        &self.table
    }

    fn len(&self) -> usize {
        self.table.len() / 3
    }

    pub(crate) fn insert(&mut self, color: Rgb) -> Result<u8, ExportError> {
        let (at, key) = self.probe(color);
        if self.keys[at] == key {
            return Ok(self.slots[at]);
        }
        let slot = self.push(color).ok_or(ExportError::TooManyColors {
            count: self.len() + 1,
            max: GIF_MAX_COLORS,
        })?;
        self.keys[at] = key;
        self.slots[at] = slot;
        Ok(slot)
    }

    pub(crate) fn get(&self, color: Rgb) -> Option<u8> {
        let (at, key) = self.probe(color);
        (self.keys[at] == key).then_some(self.slots[at])
    }

    /// A slot no color maps to, for transparency, when the table has room.
    pub(crate) fn reserve(&mut self) -> Option<u8> {
        self.push(Rgb::default())
    }

    fn push(&mut self, color: Rgb) -> Option<u8> {
        let slot = u8::try_from(self.len()).ok()?;
        self.table.extend_from_slice(&[color.r, color.g, color.b]);
        Some(slot)
    }

    /// The bucket holding `color`, or the empty one where it belongs.
    fn probe(&self, color: Rgb) -> (usize, u32) {
        let key =
            OCCUPIED | u32::from(color.r) << 16 | u32::from(color.g) << 8 | u32::from(color.b);
        let mask = CAPACITY - 1;
        let mut at = usize::try_from(key.wrapping_mul(0x9e37_79b1) >> (32 - BITS)).unwrap_or(0);
        while self.keys[at] != 0 && self.keys[at] != key {
            at = (at + 1) & mask;
        }
        (at, key)
    }
}
