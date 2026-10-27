//! Core types: paints, palettes, canvas views, time, hashing. Memory profile: strict heapless (`no_std`, no `alloc`).
#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::disallowed_macros, clippy::disallowed_types)]

mod canvas;
mod color;
mod geometry;
mod hash;
mod palette;
mod time;
mod tone;

pub use canvas::{CanvasError, CanvasMut, CanvasView, MAX_CANVAS_SIDE, Size};
pub use color::{ColorParseError, Rgb};
pub use geometry::{COORD_LIMIT, snap_coord, snap_extent};
pub use hash::hash_cells;
pub use palette::{MAX_RAMPS, MAX_TONES, Paint, Palette, PaletteError, Ramp, RampId};
pub use time::{Frame, FrameCount, FrameRate, MAX_FPS, MAX_FRAMES, TimeError};
pub use tone::{Tone, ToneName};
