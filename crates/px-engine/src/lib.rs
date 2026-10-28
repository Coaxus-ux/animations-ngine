//! Scene model and frame rendering. Memory profile: `Scene::new` and `FrameBuffer::new` allocate; `FrameBuffer::render` never allocates.
#![forbid(unsafe_code)]

mod model;
mod render;

pub use model::*;
pub use render::{FrameBuffer, FrameReport, RenderError, object_seed};
