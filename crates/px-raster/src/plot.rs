use px_core::{CanvasMut, Paint};

/// Anything shapes can be rasterized into: a canvas, a coverage mask, a test grid.
pub trait Plot {
    fn width(&self) -> i32;
    fn height(&self) -> i32;
    /// Covers row `y` from `x_start` (inclusive) to `x_end` (exclusive); out-of-range parts are ignored.
    fn span(&mut self, y: i32, x_start: i32, x_end: i32);

    fn pixel(&mut self, x: i32, y: i32) {
        self.span(y, x, x.saturating_add(1));
    }
}

pub struct Brush<'c, 'a> {
    canvas: &'c mut CanvasMut<'a>,
    paint: Paint,
}

impl<'c, 'a> Brush<'c, 'a> {
    pub fn new(canvas: &'c mut CanvasMut<'a>, paint: Paint) -> Self {
        Self { canvas, paint }
    }
}

impl Plot for Brush<'_, '_> {
    fn width(&self) -> i32 {
        i32::from(self.canvas.size().width())
    }

    fn height(&self) -> i32 {
        i32::from(self.canvas.size().height())
    }

    fn span(&mut self, y: i32, x_start: i32, x_end: i32) {
        self.canvas.fill_span(y, x_start, x_end, self.paint);
    }
}
