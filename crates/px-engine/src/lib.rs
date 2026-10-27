//! Scene model and frame rendering. Memory profile: `Scene::new` allocates; `render_frame` never allocates.
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
