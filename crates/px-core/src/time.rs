use core::fmt;
use core::num::{NonZeroU16, NonZeroU32};

pub const MAX_FPS: u16 = 120;
pub const MAX_FRAMES: u32 = 10_000;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Frame(u32);

impl Frame {
    #[must_use]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FrameRate(NonZeroU16);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FrameCount(NonZeroU32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimeError {
    InvalidFrameRate { fps: u16 },
    InvalidFrameCount { frames: u32 },
}

impl FrameRate {
    pub fn new(fps: u16) -> Result<Self, TimeError> {
        NonZeroU16::new(fps)
            .filter(|fps| fps.get() <= MAX_FPS)
            .map(Self)
            .ok_or(TimeError::InvalidFrameRate { fps })
    }

    #[must_use]
    pub const fn get(self) -> u16 {
        self.0.get()
    }

    #[must_use]
    pub const fn period(self, units_per_second: u32) -> u32 {
        let fps = self.get() as u32;
        (units_per_second + fps / 2) / fps
    }

    #[must_use]
    pub fn seconds_at(self, frame: Frame) -> f64 {
        f64::from(frame.index()) / f64::from(self.get())
    }
}

impl FrameCount {
    pub fn new(frames: u32) -> Result<Self, TimeError> {
        NonZeroU32::new(frames)
            .filter(|frames| frames.get() <= MAX_FRAMES)
            .map(Self)
            .ok_or(TimeError::InvalidFrameCount { frames })
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }

    pub fn frames(self) -> impl Iterator<Item = Frame> {
        (0..self.get()).map(Frame::new)
    }
}

impl fmt::Display for TimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFrameRate { fps } => write!(f, "fps {fps} is outside 1..={MAX_FPS}"),
            Self::InvalidFrameCount { frames } => {
                write!(f, "duration of {frames} frames is outside 1..={MAX_FRAMES}")
            }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]
mod tests {
    use super::*;

    #[test]
    fn frame_period_rounds_to_nearest() {
        let period = |fps: u16, units: u32| FrameRate::new(fps).unwrap().period(units);
        assert_eq!(period(24, 1000), 42);
        assert_eq!(period(12, 1000), 83);
        assert_eq!(period(12, 100), 8);
        assert_eq!(period(1, 1000), 1000);
        assert_eq!(period(MAX_FPS, 100), 1);
    }
}
