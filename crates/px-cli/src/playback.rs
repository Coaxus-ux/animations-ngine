use std::time::Duration;

use px_core::Frame;
use px_engine::{Looping, Timing};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Playback {
    seconds: f64,
    paused: bool,
}

impl Playback {
    pub(crate) fn advance(&mut self, delta: Duration) {
        if !self.paused {
            self.seconds += delta.as_secs_f64();
        }
    }

    pub(crate) fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub(crate) const fn is_paused(&self) -> bool {
        self.paused
    }

    pub(crate) fn restart(&mut self) {
        self.seconds = 0.0;
    }

    pub(crate) fn step(&mut self, frames: i32, timing: Timing) {
        let fps = f64::from(timing.fps.get());
        let current = f64::from(self.frame(timing).index());
        self.seconds = ((current + f64::from(frames)).max(0.0) + 0.5) / fps;
    }

    pub(crate) fn frame(&self, timing: Timing) -> Frame {
        let duration = u64::from(timing.duration.get());
        let index = (self.seconds * f64::from(timing.fps.get()))
            .floor()
            .max(0.0);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "non-negative and clamped to u32 below"
        )]
        let index = index.min(f64::from(u32::MAX)) as u64;
        let index = match timing.looping {
            Looping::Loop => index % duration,
            Looping::Once => index.min(duration - 1),
        };
        Frame::new(u32::try_from(index).unwrap_or(u32::MAX))
    }
}

#[cfg(test)]
#[path = "playback_tests.rs"]
mod tests;
