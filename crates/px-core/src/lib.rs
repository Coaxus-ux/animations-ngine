#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::disallowed_macros, clippy::disallowed_types)]

mod canvas;
mod color;
mod dither;
mod float;
mod geometry;
mod hash;
mod hsv;
mod palette;
mod seed;
mod time;
mod tone;

pub use canvas::{CanvasError, CanvasMut, CanvasView, MAX_CANVAS_SIDE, Size};
pub use color::{ColorParseError, Rgb};
pub use dither::{bayer_threshold, dither_keep, dither_steps};
pub use float::floor;
pub use geometry::{COORD_LIMIT, snap_coord, snap_extent};
pub use hash::{hash_bytes, hash_cells};
pub use hsv::{Hsv, ramp as hsv_ramp};
pub use palette::{
    LightColor, MAX_LIGHT_COLORS, MAX_RAMPS, MAX_TONES, Paint, Palette, PaletteError, Ramp, RampId,
};
pub use seed::{absorb, hash_seed, mix, seeded};
pub use time::{Frame, FrameCount, FrameRate, MAX_FPS, MAX_FRAMES, TimeError};
pub use tone::{Tone, ToneName};
