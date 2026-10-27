//! Rasterizes shapes into an indexed canvas with pixel-art rules: integer coordinates,
//! no anti-aliasing, 8-connected one-pixel lines.
//!
//! # Allocation
//!
//! Strict heapless: `no_std`, no `alloc`. Work is bounded by the canvas size and by
//! [`px_core::COORD_LIMIT`].
#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::disallowed_macros, clippy::disallowed_types)]

mod ellipse;
mod line;
mod rect;

pub use ellipse::fill_ellipse;
pub use line::draw_line;
pub use rect::{Rect, fill_rect};

/// A pixel position.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Point {
    /// Column; 0 is the left edge.
    pub x: i32,
    /// Row; 0 is the top edge.
    pub y: i32,
}
