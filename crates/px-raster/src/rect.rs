use px_core::{CanvasMut, Paint};

/// An axis-aligned rectangle: top-left corner plus size. A size ≤ 0 is empty.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Rect {
    /// Left column.
    pub x: i32,
    /// Top row.
    pub y: i32,
    /// Width in pixels.
    pub width: i32,
    /// Height in pixels.
    pub height: i32,
}

impl Rect {
    /// Rows covered, clipped to the canvas height.
    pub(crate) fn rows(self, canvas_height: u16) -> core::ops::Range<i32> {
        let end = self.y.saturating_add(self.height.max(0));
        self.y.max(0)..end.min(i32::from(canvas_height))
    }

    /// Exclusive right edge.
    pub(crate) const fn right(self) -> i32 {
        self.x
            .saturating_add(if self.width > 0 { self.width } else { 0 })
    }
}

/// Fills a rectangle, clipped to the canvas.
pub fn fill_rect(canvas: &mut CanvasMut<'_>, rect: Rect, paint: Paint) {
    for y in rect.rows(canvas.size().height()) {
        canvas.fill_span(y, rect.x, rect.right(), paint);
    }
}
