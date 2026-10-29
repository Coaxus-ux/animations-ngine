//! Sequences: shots of scenes joined by transitions, composed into frames (DIRECTOR.md §4).
//! Memory profile: `Sequence::new` and `Player::new` allocate; `Player::render` never allocates.
#![forbid(unsafe_code)]

mod error;
mod player;
mod sequence;
mod transition;

pub use error::SequenceError;
pub use player::{Player, SequenceView};
pub use sequence::{Edge, MAX_SHOTS, Sequence, Shot, Transition, TransitionKind};
