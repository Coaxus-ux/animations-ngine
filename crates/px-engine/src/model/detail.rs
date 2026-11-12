use core::fmt;

use px_core::{Size, snap_extent};

pub const MAX_DETAIL: u8 = 8;

const THOUSANDTHS: u16 = 1000;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Detail(u16);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DetailError(pub f64);

impl Detail {
    pub const NATIVE: Self = Self(THOUSANDTHS);

    pub fn new(factor: f64) -> Result<Self, DetailError> {
        if !(factor.is_finite() && (1.0..=f64::from(MAX_DETAIL)).contains(&factor)) {
            return Err(DetailError(factor));
        }
        u16::try_from(snap_extent(factor * f64::from(THOUSANDTHS)))
            .map(Self)
            .map_err(|_| DetailError(factor))
    }

    #[must_use]
    pub fn whole(factor: u8) -> Self {
        Self(u16::from(factor.clamp(1, MAX_DETAIL)) * THOUSANDTHS)
    }

    #[must_use]
    pub fn get(self) -> f64 {
        f64::from(self.0) / f64::from(THOUSANDTHS)
    }

    #[must_use]
    pub fn grain(self) -> i32 {
        i32::from(self.0 / THOUSANDTHS)
    }

    #[must_use]
    pub fn scale(self, size: Size) -> Option<Size> {
        let scaled = |side: u16| u16::try_from(snap_extent(f64::from(side) * self.get())).ok();
        Size::new(scaled(size.width())?, scaled(size.height())?).ok()
    }
}

impl Default for Detail {
    fn default() -> Self {
        Self::NATIVE
    }
}

impl fmt::Display for Detail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get())
    }
}

impl fmt::Display for DetailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "detail must be a number from 1 to {MAX_DETAIL}, found {}",
            self.0
        )
    }
}

impl std::error::Error for DetailError {}
