/// Largest supported upscale factor.
pub const MAX_SCALE: u8 = 16;

/// An integer upscale factor (1..=[`MAX_SCALE`]); pixels are never resampled.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Scale(u8);

impl Scale {
    /// Validates a scale factor; `None` outside 1..=[`MAX_SCALE`].
    #[must_use]
    pub const fn new(factor: u8) -> Option<Self> {
        if factor >= 1 && factor <= MAX_SCALE {
            Some(Self(factor))
        } else {
            None
        }
    }

    /// The factor.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }

    /// `value × factor` as `u16`, or `None` on overflow.
    pub(crate) fn apply(self, value: u16) -> Option<u16> {
        value.checked_mul(u16::from(self.0))
    }
}
