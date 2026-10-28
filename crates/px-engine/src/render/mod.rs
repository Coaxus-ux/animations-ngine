mod buffer;
mod composite;
mod frame;
mod geometry;
mod lighting;
mod mask;
mod placement;
mod reflection;
mod report;
mod surface;
mod texture;
mod transform;
mod values;

use std::fmt;

use px_core::Size;
use px_expr::EvalError;

pub use buffer::FrameBuffer;
pub use frame::object_seed;
pub use report::FrameReport;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderError {
    CanvasSize { expected: Size, actual: Size },
    Eval { shape: usize, error: EvalError },
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CanvasSize { expected, actual } => write!(
                f,
                "canvas is {}×{}, scene needs {}×{}",
                actual.width(),
                actual.height(),
                expected.width(),
                expected.height()
            ),
            Self::Eval { shape, error } => write!(f, "shape #{shape}: {error}"),
        }
    }
}

impl std::error::Error for RenderError {}
