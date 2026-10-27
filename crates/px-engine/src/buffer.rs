use px_core::{CanvasMut, CanvasView, Frame, Paint, Size};

use crate::model::Scene;
use crate::render::{RenderError, render_frame};

#[derive(Clone, Debug)]
pub struct FrameBuffer {
    size: Size,
    cells: Vec<Paint>,
}

impl FrameBuffer {
    #[must_use]
    pub fn new(scene: &Scene) -> Self {
        Self {
            size: scene.size(),
            cells: vec![scene.background(); scene.size().cell_count()],
        }
    }

    pub fn render(&mut self, scene: &Scene, frame: Frame) -> Result<CanvasView<'_>, RenderError> {
        let mismatch = RenderError::CanvasSize {
            expected: scene.size(),
            actual: self.size,
        };
        let mut canvas = CanvasMut::new(self.size, &mut self.cells).map_err(|_| mismatch)?;
        render_frame(scene, frame, &mut canvas)?;
        CanvasView::new(self.size, &self.cells).map_err(|_| mismatch)
    }
}
