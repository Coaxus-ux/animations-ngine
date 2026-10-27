use std::fmt;

use px_core::{CanvasMut, Frame, Size};
use px_expr::{Env, EvalError};

use crate::model::{Scene, Timing};

/// Why a frame could not be rendered.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderError {
    /// The canvas size differs from the scene size.
    CanvasSize {
        /// The scene's size.
        expected: Size,
        /// The canvas's size.
        actual: Size,
    },
    /// An expression failed.
    Eval {
        /// Index of the shape whose parameter failed.
        shape: usize,
        /// The evaluation error.
        error: EvalError,
    },
}

/// Renders one frame into a caller-owned canvas.
///
/// Frames past the scene's duration are allowed; a looping scene renders frame
/// `duration` identically to frame 0.
///
/// # Allocation
///
/// No heap allocation (verified by `tests/no_alloc.rs`).
///
/// # Errors
///
/// [`RenderError::CanvasSize`] or [`RenderError::Eval`].
pub fn render_frame(
    scene: &Scene,
    frame: Frame,
    canvas: &mut CanvasMut<'_>,
) -> Result<(), RenderError> {
    if canvas.size() != scene.size() {
        return Err(RenderError::CanvasSize {
            expected: scene.size(),
            actual: canvas.size(),
        });
    }
    let env = env_at(scene.timing(), frame);
    canvas.fill(scene.background());
    for (index, shape) in scene.shapes().iter().enumerate() {
        shape
            .geometry
            .draw(canvas, &env, shape.paint)
            .map_err(|error| RenderError::Eval {
                shape: index,
                error,
            })?;
    }
    Ok(())
}

fn env_at(timing: Timing, frame: Frame) -> Env {
    let fps = timing.fps;
    Env {
        time: fps.seconds_at(frame),
        frame: f64::from(frame.index()),
        fps: f64::from(fps.get()),
        duration: fps.seconds_at(Frame::new(timing.duration.get())),
    }
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
