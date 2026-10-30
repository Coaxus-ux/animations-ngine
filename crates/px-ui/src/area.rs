pub use px_raster::Rect;

#[must_use]
pub const fn rect(x: i32, y: i32, width: i32, height: i32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

/// Layout by cutting rectangles: every split returns the piece taken and the rest.
pub trait Area: Sized {
    fn contains(self, x: i32, y: i32) -> bool;
    fn right(self) -> i32;
    fn bottom(self) -> i32;
    #[must_use]
    fn inset(self, by: i32) -> Self;
    #[must_use]
    fn intersect(self, other: Self) -> Self;
    fn take_left(self, width: i32) -> (Self, Self);
    fn take_right(self, width: i32) -> (Self, Self);
    fn take_top(self, height: i32) -> (Self, Self);
    fn take_bottom(self, height: i32) -> (Self, Self);
    /// A `width`×`height` box centered in this one.
    #[must_use]
    fn centered(self, width: i32, height: i32) -> Self;
    fn is_empty(&self) -> bool;
}

impl Area for Rect {
    fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.right() && y < self.bottom()
    }

    fn right(self) -> i32 {
        self.x + self.width.max(0)
    }

    fn bottom(self) -> i32 {
        self.y + self.height.max(0)
    }

    fn inset(self, by: i32) -> Self {
        rect(
            self.x + by,
            self.y + by,
            (self.width - 2 * by).max(0),
            (self.height - 2 * by).max(0),
        )
    }

    fn intersect(self, other: Self) -> Self {
        let (x, y) = (self.x.max(other.x), self.y.max(other.y));
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        rect(x, y, (right - x).max(0), (bottom - y).max(0))
    }

    fn take_left(self, width: i32) -> (Self, Self) {
        let width = width.clamp(0, self.width.max(0));
        (
            rect(self.x, self.y, width, self.height),
            rect(self.x + width, self.y, self.width - width, self.height),
        )
    }

    fn take_right(self, width: i32) -> (Self, Self) {
        let width = width.clamp(0, self.width.max(0));
        (
            rect(self.right() - width, self.y, width, self.height),
            rect(self.x, self.y, self.width - width, self.height),
        )
    }

    fn take_top(self, height: i32) -> (Self, Self) {
        let height = height.clamp(0, self.height.max(0));
        (
            rect(self.x, self.y, self.width, height),
            rect(self.x, self.y + height, self.width, self.height - height),
        )
    }

    fn take_bottom(self, height: i32) -> (Self, Self) {
        let height = height.clamp(0, self.height.max(0));
        (
            rect(self.x, self.bottom() - height, self.width, height),
            rect(self.x, self.y, self.width, self.height - height),
        )
    }

    fn centered(self, width: i32, height: i32) -> Self {
        rect(
            self.x + (self.width - width) / 2,
            self.y + (self.height - height) / 2,
            width,
            height,
        )
    }

    fn is_empty(&self) -> bool {
        self.width <= 0 || self.height <= 0
    }
}
