use px_core::{CanvasView, Frame, Paint, Size};

use super::RenderError;
use super::frame::render;
use super::mask::Mask;
use super::report::FrameReport;
use super::surface::Surface;
use crate::model::Scene;

/// Every per-pixel buffer a frame needs, allocated once; `render` never allocates.
#[derive(Clone, Debug)]
pub struct FrameBuffer {
    size: Size,
    cells: Vec<Paint>,
    depth: Vec<u16>,
    flags: Vec<u8>,
    coverage: Vec<u8>,
    temp: Vec<u8>,
    distance: Vec<u16>,
    noise: Vec<i8>,
    report: FrameReport,
}

impl FrameBuffer {
    #[must_use]
    pub fn new(scene: &Scene) -> Self {
        let count = scene.size().cell_count();
        Self {
            size: scene.size(),
            cells: vec![scene.background(); count],
            depth: vec![0; count],
            flags: vec![0; count],
            coverage: vec![0; count],
            temp: vec![0; count],
            distance: vec![0; count],
            noise: vec![0; count],
            report: FrameReport::default(),
        }
    }

    pub fn render(&mut self, scene: &Scene, frame: Frame) -> Result<CanvasView<'_>, RenderError> {
        let mismatch = RenderError::CanvasSize {
            expected: scene.size(),
            actual: self.size,
        };
        if scene.size() != self.size {
            return Err(mismatch);
        }
        let (width, height) = (i32::from(self.size.width()), i32::from(self.size.height()));
        let mut surface = Surface {
            width,
            height,
            cells: &mut self.cells,
            depth: &mut self.depth,
            flags: &mut self.flags,
        };
        let mut mask = Mask {
            width,
            height,
            coverage: &mut self.coverage,
            temp: &mut self.temp,
            distance: &mut self.distance,
            noise: &mut self.noise,
            bbox: None,
        };
        mask.coverage.fill(0);
        self.report = FrameReport::default();
        render(scene, frame, &mut surface, &mut mask, &mut self.report)?;
        CanvasView::new(self.size, &self.cells).map_err(|_| mismatch)
    }

    #[must_use]
    pub const fn report(&self) -> FrameReport {
        self.report
    }
}
