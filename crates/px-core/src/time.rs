use core::fmt;
use core::num::{NonZeroU16, NonZeroU32};

/// Highest supported frame rate.
pub const MAX_FPS: u16 = 120;
/// Longest supported scene, in frames.
pub const MAX_FRAMES: u32 = 10_000;

/// A frame index. Time is counted in whole frames so loops are exact.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Frame(u32);

impl Frame {
    /// Creates a frame index.
    #[must_use]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// The index.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// Frames per second (1..=[`MAX_FPS`]).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FrameRate(NonZeroU16);

/// A positive number of frames (1..=[`MAX_FRAMES`]).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FrameCount(NonZeroU32);

/// Why a time value is invalid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimeError {
    /// The frame rate is 0 or above [`MAX_FPS`].
    InvalidFrameRate {
        /// Requested frames per second.
        fps: u16,
    },
    /// The frame count is 0 or above [`MAX_FRAMES`].
    InvalidFrameCount {
        /// Requested frames.
        frames: u32,
    },
}

impl FrameRate {
    /// Validates a frame rate.
    ///
    /// # Errors
    ///
    /// [`TimeError::InvalidFrameRate`] outside 1..=[`MAX_FPS`].
    pub fn new(fps: u16) -> Result<Self, TimeError> {
        NonZeroU16::new(fps)
            .filter(|fps| fps.get() <= MAX_FPS)
            .map(Self)
            .ok_or(TimeError::InvalidFrameRate { fps })
    }

    /// Frames per second.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0.get()
    }

    /// The time of a frame in seconds.
    #[must_use]
    pub fn seconds_at(self, frame: Frame) -> f64 {
        f64::from(frame.index()) / f64::from(self.get())
    }
}

impl FrameCount {
    /// Validates a frame count.
    ///
    /// # Errors
    ///
    /// [`TimeError::InvalidFrameCount`] outside 1..=[`MAX_FRAMES`].
    pub fn new(frames: u32) -> Result<Self, TimeError> {
        NonZeroU32::new(frames)
            .filter(|frames| frames.get() <= MAX_FRAMES)
            .map(Self)
            .ok_or(TimeError::InvalidFrameCount { frames })
    }

    /// Number of frames.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }

    /// Every frame from 0 up to, not including, this count.
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
