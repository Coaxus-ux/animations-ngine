use px_core::{Frame, FrameCount, FrameRate, Size};
use px_engine::{Looping, Scene};

use crate::error::SequenceError;

pub const MAX_SHOTS: usize = 256;

#[derive(Clone, Debug)]
pub struct Shot {
    pub scene: Scene,
    pub frames: FrameCount,
    pub from: Frame,
    pub speed: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionKind {
    Cut,
    Dither,
    Black,
    White,
    Wipe(Edge),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Transition {
    pub kind: TransitionKind,
    pub frames: u32,
}

impl Transition {
    pub const CUT: Self = Self {
        kind: TransitionKind::Cut,
        frames: 0,
    };
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Moment {
    Shot {
        shot: usize,
        frame: Frame,
    },
    Between {
        from: (usize, Frame),
        to: (usize, Frame),
        kind: TransitionKind,
        progress: f64,
    },
}

#[derive(Clone, Debug)]
pub struct Sequence {
    name: String,
    shots: Vec<Shot>,
    transitions: Vec<Transition>,
    starts: Vec<u32>,
    duration: FrameCount,
}

impl Sequence {
    pub fn new(
        name: String,
        shots: Vec<Shot>,
        transitions: Vec<Transition>,
    ) -> Result<Self, SequenceError> {
        let first = shots.first().ok_or(SequenceError::NoShots)?;
        if shots.len() > MAX_SHOTS {
            return Err(SequenceError::TooManyShots {
                count: shots.len(),
                max: MAX_SHOTS,
            });
        }
        if transitions.len() + 1 != shots.len() {
            return Err(SequenceError::TransitionCount {
                shots: shots.len(),
                transitions: transitions.len(),
            });
        }
        let (size, fps) = (first.scene.size(), first.scene.timing().fps);
        for (index, shot) in shots.iter().enumerate() {
            check_shot(index, shot, size, fps)?;
        }
        let overlap = |index: Option<usize>| {
            index
                .and_then(|i| transitions.get(i))
                .map_or(0, |t| t.frames)
        };
        let mut starts = Vec::with_capacity(shots.len());
        let mut start = 0u32;
        for (index, shot) in shots.iter().enumerate() {
            let (incoming, outgoing) = (overlap(index.checked_sub(1)), overlap(Some(index)));
            if incoming.saturating_add(outgoing) >= shot.frames.get() {
                return Err(SequenceError::TransitionTooLong { shot: index });
            }
            starts.push(start);
            start = start.saturating_add(shot.frames.get() - outgoing);
        }
        let duration = FrameCount::new(start).map_err(SequenceError::Length)?;
        Ok(Self {
            name,
            shots,
            transitions,
            starts,
            duration,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn shots(&self) -> &[Shot] {
        &self.shots
    }

    #[must_use]
    pub fn transitions(&self) -> &[Transition] {
        &self.transitions
    }

    #[must_use]
    pub fn size(&self) -> Size {
        self.shots[0].scene.size()
    }

    #[must_use]
    pub fn fps(&self) -> FrameRate {
        self.shots[0].scene.timing().fps
    }

    #[must_use]
    pub const fn duration(&self) -> FrameCount {
        self.duration
    }

    #[must_use]
    pub fn starts(&self) -> &[u32] {
        &self.starts
    }

    pub(crate) fn moment(&self, frame: Frame) -> Moment {
        let at = frame.index().min(self.duration.get() - 1);
        let shot = self.starts.partition_point(|start| *start <= at) - 1;
        let local = |shot: usize| self.scene_frame(shot, at - self.starts[shot]);
        let Some(previous) = shot.checked_sub(1) else {
            return Moment::Shot {
                shot,
                frame: local(shot),
            };
        };
        let transition = self.transitions[previous];
        let into = at - self.starts[shot];
        if into >= transition.frames {
            return Moment::Shot {
                shot,
                frame: local(shot),
            };
        }
        Moment::Between {
            from: (previous, local(previous)),
            to: (shot, local(shot)),
            kind: transition.kind,
            progress: (f64::from(into) + 1.0) / (f64::from(transition.frames) + 1.0),
        }
    }

    fn scene_frame(&self, shot: usize, local: u32) -> Frame {
        let shot = &self.shots[shot];
        let timing = shot.scene.timing();
        let length = timing.duration.get();
        let advanced = px_core::floor(f64::from(local) * shot.speed);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "non-negative and clamped below u32::MAX"
        )]
        let advanced = advanced.clamp(0.0, f64::from(u32::MAX)) as u32;
        let raw = shot.from.index().saturating_add(advanced);
        Frame::new(match timing.looping {
            Looping::Loop => raw % length,
            Looping::Once => raw.min(length - 1),
        })
    }
}

fn check_shot(index: usize, shot: &Shot, size: Size, fps: FrameRate) -> Result<(), SequenceError> {
    if shot.scene.size() != size {
        return Err(SequenceError::Mismatch {
            shot: index,
            what: "size",
        });
    }
    if shot.scene.timing().fps != fps {
        return Err(SequenceError::Mismatch {
            shot: index,
            what: "fps",
        });
    }
    if !(shot.speed.is_finite() && shot.speed > 0.0) {
        return Err(SequenceError::Speed { shot: index });
    }
    Ok(())
}
