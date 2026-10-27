use px_core::{CanvasMut, Paint};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub(crate) fn rows(self, canvas_height: u16) -> core::ops::Range<i32> {
        let end = self.y.saturating_add(self.height.max(0));
        self.y.max(0)..end.min(i32::from(canvas_height))
    }

    pub(crate) const fn right(self) -> i32 {
        self.x
            .saturating_add(if self.width > 0 { self.width } else { 0 })
    }
}

pub fn fill_rect(canvas: &mut CanvasMut<'_>, rect: Rect, paint: Paint) {
    for y in rect.rows(canvas.size().height()) {
        canvas.fill_span(y, rect.x, rect.right(), paint);
    }
}
