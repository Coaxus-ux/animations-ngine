use px_core::COORD_LIMIT;

use crate::Plot;

use crate::rect::Rect;

pub fn fill_ellipse(plot: &mut impl Plot, bounds: Rect) {
    let bounds = Rect {
        width: bounds.width.min(COORD_LIMIT),
        height: bounds.height.min(COORD_LIMIT),
        ..bounds
    };
    if bounds.width <= 0 || bounds.height <= 0 {
        return;
    }
    let shape = Doubled::new(bounds);
    for y in bounds.rows(plot.height()) {
        if let Some((first, last)) = shape.span(y) {
            plot.span(y, saturate(first), saturate(last).saturating_add(1));
        }
    }
}

// Doubled coordinates: pixel `p` has its center at `2p + 1`, so even and odd sizes stay exact.
struct Doubled {
    width: i64,
    height: i64,
    center_x: i64,
    center_y: i64,
    limit: i64,
}

impl Doubled {
    fn new(bounds: Rect) -> Self {
        let width = i64::from(bounds.width);
        let height = i64::from(bounds.height);
        Self {
            width,
            height,
            center_x: 2 * i64::from(bounds.x) + width,
            center_y: 2 * i64::from(bounds.y) + height,
            limit: (width * height) * (width * height),
        }
    }

    fn span(&self, y: i32) -> Option<(i64, i64)> {
        let dy = 2 * i64::from(y) + 1 - self.center_y;
        debug_assert!(dy.abs() <= self.height, "row outside the ellipse bounds");
        let row_term = dy * dy * self.width * self.width;
        if row_term > self.limit {
            return None;
        }
        let half = self.widest_half(row_term);
        let first = (self.center_x - half).div_euclid(2);
        let last = (self.center_x + half - 1).div_euclid(2);
        (first <= last).then_some((first, last))
    }

    fn widest_half(&self, row_term: i64) -> i64 {
        let fits = |half: i64| half * half * self.height * self.height + row_term <= self.limit;
        let (mut low, mut high) = (0, self.width);
        while low < high {
            let mid = low + (high - low + 1) / 2;
            if fits(mid) { low = mid } else { high = mid - 1 }
        }
        low
    }
}

fn saturate(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
}
