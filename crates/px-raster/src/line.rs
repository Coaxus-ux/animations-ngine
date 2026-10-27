use px_core::{COORD_LIMIT, CanvasMut, Paint};

use crate::Point;

pub fn draw_line(canvas: &mut CanvasMut<'_>, from: Point, to: Point, paint: Paint) {
    let (mut x, mut y) = (clamp(from.x), clamp(from.y));
    let (end_x, end_y) = (clamp(to.x), clamp(to.y));
    let dx = (end_x - x).abs();
    let dy = -(end_y - y).abs();
    let step_x = if x < end_x { 1 } else { -1 };
    let step_y = if y < end_y { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        canvas.set(x, y, paint);
        if x == end_x && y == end_y {
            return;
        }
        let doubled = 2 * err;
        if doubled >= dy {
            err += dy;
            x += step_x;
        }
        if doubled <= dx {
            err += dx;
            y += step_y;
        }
    }
}

const fn clamp(value: i32) -> i32 {
    if value < -COORD_LIMIT {
        -COORD_LIMIT
    } else if value > COORD_LIMIT {
        COORD_LIMIT
    } else {
        value
    }
}
