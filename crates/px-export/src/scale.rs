pub const MAX_SCALE: u8 = 16;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Scale(u8);

impl Scale {
    #[must_use]
    pub const fn new(factor: u8) -> Option<Self> {
        if factor >= 1 && factor <= MAX_SCALE {
            Some(Self(factor))
        } else {
            None
        }
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }

    pub(crate) fn apply(self, value: u16) -> Option<u16> {
        value.checked_mul(u16::from(self.0))
    }
}
