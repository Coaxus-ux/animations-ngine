//! The runtime scene model and frame rendering.
//!
//! A [`Scene`] is built once from a validated [`SceneSpec`] (the authoring side, which
//! may allocate). [`render_frame`] then turns `(scene, frame)` into indexed pixels.
//!
//! # Allocation
//!
//! [`Scene::new`] allocates (allocation-conscious). [`render_frame`] is allocation-free
//! in steady state: it only evaluates compiled expressions and writes into the
//! caller-owned canvas.
#![forbid(unsafe_code)]

mod buffer;
mod draw;
mod model;
mod render;

pub use buffer::FrameBuffer;
pub use model::{
    Bounds, Geometry, Looping, MAX_SHAPES, ModelError, Param, Scene, SceneSpec, Segment, Shape,
    Timing,
};
pub use render::{RenderError, render_frame};
