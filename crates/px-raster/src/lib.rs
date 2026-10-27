//! Pixel-art rasterization into an indexed canvas. Memory profile: strict heapless (`no_std`, no `alloc`).
#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::disallowed_macros, clippy::disallowed_types)]

mod ellipse;
mod line;
mod rect;

pub use ellipse::fill_ellipse;
pub use line::draw_line;
pub use rect::{Rect, fill_rect};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}
