use crate::Plot;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub(crate) fn rows(self, canvas_height: i32) -> core::ops::Range<i32> {
        let end = self.y.saturating_add(self.height.max(0));
        self.y.max(0)..end.min(canvas_height)
    }

    pub(crate) const fn right(self) -> i32 {
        self.x
            .saturating_add(if self.width > 0 { self.width } else { 0 })
    }
}

pub fn fill_rect(plot: &mut impl Plot, rect: Rect) {
    for y in rect.rows(plot.height()) {
        plot.span(y, rect.x, rect.right());
    }
}
