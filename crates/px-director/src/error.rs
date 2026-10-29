use std::fmt;

use px_core::TimeError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SequenceError {
    NoShots,
    TooManyShots { count: usize, max: usize },
    TransitionCount { shots: usize, transitions: usize },
    Mismatch { shot: usize, what: &'static str },
    Speed { shot: usize },
    TransitionTooLong { shot: usize },
    Length(TimeError),
}

impl fmt::Display for SequenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoShots => write!(f, "a sequence needs at least one shot"),
            Self::TooManyShots { count, max } => {
                write!(f, "a sequence has at most {max} shots, found {count}")
            }
            Self::TransitionCount { shots, transitions } => write!(
                f,
                "{shots} shots need {} transitions between them, found {transitions}",
                shots - 1
            ),
            Self::Mismatch { shot, what } => write!(
                f,
                "shot {} has another {what} than the first shot; every shot needs the same",
                shot + 1
            ),
            Self::Speed { shot } => write!(f, "shot {} needs a speed above 0", shot + 1),
            Self::TransitionTooLong { shot } => write!(
                f,
                "the transitions into and out of shot {} are as long as the shot; make it longer",
                shot + 1
            ),
            Self::Length(error) => write!(f, "sequence length: {error}"),
        }
    }
}

impl std::error::Error for SequenceError {}
