use px_core::{CanvasView, Frame, Paint, Size};

use super::RenderError;
use super::frame::render;
use super::mask::Mask;
use super::placement::Placed;
use super::report::FrameReport;
use super::shadow::{Memo, memo_cells};
use super::stamp::{Capture, Stamps};
use super::surface::{Surface, Target};
use crate::model::{MAX_INSTANCES, Scene};

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
    paints: Vec<Paint>,
    placed: Vec<Placed>,
    memo: Vec<Memo>,
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
            paints: vec![scene.background(); count],
            placed: vec![Placed::root(px_expr::Env::default()); MAX_INSTANCES],
            memo: vec![Memo::EMPTY; memo_cells(usize::from(scene.size().width()))],
            report: FrameReport::default(),
        }
    }

    pub fn render(&mut self, scene: &Scene, frame: Frame) -> Result<CanvasView<'_>, RenderError> {
        self.draw(scene, frame, None)?;
        CanvasView::new(self.size, &self.cells).map_err(|_| RenderError::CanvasSize {
            expected: scene.size(),
            actual: self.size,
        })
    }

    pub(crate) fn capture(scene: &Scene) -> Stamps {
        let mut capture = Capture::new(scene.shapes().len());
        match Self::new(scene).draw(scene, Frame::new(0), Some(&mut capture)) {
            Ok(()) => capture.into_stamps(),
            Err(_) => Stamps::default(),
        }
    }

    fn draw(
        &mut self,
        scene: &Scene,
        frame: Frame,
        capture: Option<&mut Capture>,
    ) -> Result<(), RenderError> {
        if scene.size() != self.size {
            return Err(RenderError::CanvasSize {
                expected: scene.size(),
                actual: self.size,
            });
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
            paints: &mut self.paints,
            bbox: None,
        };
        mask.coverage.fill(0);
        self.report = FrameReport::default();
        let target = Target {
            surface: &mut surface,
            mask: &mut mask,
            placed: &mut self.placed,
            memo: &mut self.memo,
        };
        render(scene, frame, target, &mut self.report, capture)
    }

    #[must_use]
    pub fn cells(&self) -> &[Paint] {
        &self.cells
    }

    #[must_use]
    pub fn shape_at(&self, x: i32, y: i32) -> Option<usize> {
        let (width, height) = (i32::from(self.size.width()), i32::from(self.size.height()));
        if x < 0 || y < 0 || x >= width || y >= height {
            return None;
        }
        let depth = self.depth[usize::try_from(y * width + x).ok()?];
        usize::from(depth).checked_sub(1)
    }

    #[must_use]
    pub const fn report(&self) -> FrameReport {
        self.report
    }
}
