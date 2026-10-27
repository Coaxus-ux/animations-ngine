use std::fmt;

use px_core::{CanvasMut, Frame, Size};
use px_expr::{Env, EvalError};

use crate::model::{Scene, Timing};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderError {
    CanvasSize { expected: Size, actual: Size },
    Eval { shape: usize, error: EvalError },
}

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
