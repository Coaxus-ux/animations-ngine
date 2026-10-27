use std::fmt;

use px_core::{CanvasMut, Frame, Size, hash_seed};
use px_expr::{Env, EvalError};

use crate::model::{Looping, Scene};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderError {
    CanvasSize { expected: Size, actual: Size },
    Eval { shape: usize, error: EvalError },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FrameReport {
    pub non_finite_values: u32,
    pub first_non_finite_shape: Option<usize>,
}

impl FrameReport {
    pub(crate) fn record_non_finite(&mut self, shape: usize) {
        self.non_finite_values = self.non_finite_values.saturating_add(1);
        self.first_non_finite_shape.get_or_insert(shape);
    }
}

/// Never allocates (`tests/no_alloc.rs`). Non-finite parameter values become 0 and are
/// counted in the returned report (FORMAT §9.8).
pub fn render_frame(
    scene: &Scene,
    frame: Frame,
    canvas: &mut CanvasMut<'_>,
) -> Result<FrameReport, RenderError> {
    if canvas.size() != scene.size() {
        return Err(RenderError::CanvasSize {
            expected: scene.size(),
            actual: canvas.size(),
        });
    }
    let mut report = FrameReport::default();
    let scene_env = env_at(scene, frame);
    canvas.fill(scene.background());
    for (index, shape) in scene.shapes().iter().enumerate() {
        let env = Env {
            seed: hash_seed(scene.seed(), &[u64::try_from(index).unwrap_or(u64::MAX)]),
            ..scene_env
        };
        shape
            .geometry
            .draw(canvas, &env, shape.paint, &mut |error| match error {
                EvalError::NonFinite => {
                    report.record_non_finite(index);
                    Ok(0.0)
                }
                EvalError::Malformed => Err(RenderError::Eval {
                    shape: index,
                    error,
                }),
            })?;
    }
    Ok(report)
}

fn env_at(scene: &Scene, frame: Frame) -> Env {
    let timing = scene.timing();
    let duration = timing.fps.seconds_at(Frame::new(timing.duration.get()));
    Env {
        time: timing.fps.seconds_at(frame),
        frame: f64::from(frame.index()),
        fps: f64::from(timing.fps.get()),
        duration,
        loop_period: (timing.looping == Looping::Loop).then_some(duration),
        ..Env::default()
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
