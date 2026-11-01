#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::disallowed_macros, clippy::disallowed_types)]

mod ellipse;
mod line;
mod plot;
mod rect;
mod sdf;

pub use ellipse::fill_ellipse;
pub use line::draw_line;
pub use plot::{Brush, Plot};
pub use rect::{Rect, fill_rect};
pub use sdf::{
    MAX_BLOB_CIRCLES, MAX_POLYGON_POINTS, fill_blob, fill_capsule, fill_polygon, fill_rounded_rect,
    fill_sdf,
};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}
