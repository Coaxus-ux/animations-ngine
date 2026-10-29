use px_core::COORD_LIMIT;

use crate::{Plot, Rect};

pub const MAX_POLYGON_POINTS: usize = 32;
pub const MAX_BLOB_CIRCLES: usize = 16;
/// Pixel centers often sit exactly on an edge (distance 0); float noise of 1e-15 must not
/// decide them, or animated shapes break perfect loops. The edge test uses this margin.
const EDGE_TOLERANCE: f64 = 1e-6;
/// Relative band on squared distances inside which `within` defers to `hypot`. It dwarfs
/// float error (~1e-16), so the shortcut never changes a pixel.
const SQUARED_MARGIN: f64 = 1e-9;

/// Covers every pixel whose center has `distance(x, y) <= 0` (within `EDGE_TOLERANCE`), scanning only `bbox`.
pub fn fill_sdf(plot: &mut impl Plot, bbox: Rect, distance: impl Fn(f64, f64) -> f64) {
    fill_where(plot, bbox, |x, y| distance(x, y) <= EDGE_TOLERANCE);
}

fn fill_where(plot: &mut impl Plot, bbox: Rect, inside: impl Fn(f64, f64) -> bool) {
    let x_end = bbox.right().min(plot.width());
    let x_start = bbox.x.max(0);
    for y in bbox.rows(plot.height()) {
        let mut run: Option<i32> = None;
        for x in x_start..=x_end {
            let inside = x < x_end && inside(f64::from(x) + 0.5, f64::from(y) + 0.5);
            match (inside, run) {
                (true, None) => run = Some(x),
                (false, Some(start)) => {
                    plot.span(y, start, x);
                    run = None;
                }
                _ => {}
            }
        }
    }
}

pub fn fill_rounded_rect(plot: &mut impl Plot, rect: Rect, radius: f64) {
    let (hw, hh) = (f64::from(rect.width) / 2.0, f64::from(rect.height) / 2.0);
    let (cx, cy) = (f64::from(rect.x) + hw, f64::from(rect.y) + hh);
    let radius = radius.clamp(0.0, hw.min(hh));
    fill_sdf(plot, rect, |x, y| {
        let qx = libm::fabs(x - cx) - (hw - radius);
        let qy = libm::fabs(y - cy) - (hh - radius);
        libm::hypot(qx.max(0.0), qy.max(0.0)) + qx.max(qy).min(0.0) - radius
    });
}

pub fn fill_capsule(plot: &mut impl Plot, from: [f64; 2], to: [f64; 2], radius: f64) {
    let bbox = bounding(&[from, to], radius);
    fill_where(plot, bbox, |x, y| {
        within(segment_offset([x, y], from, to), radius)
    });
}

pub fn fill_polygon(plot: &mut impl Plot, points: &[[f64; 2]]) {
    let points = &points[..points.len().min(MAX_POLYGON_POINTS)];
    if points.len() < 3 {
        return;
    }
    fill_where(plot, bounding(points, 0.0), |x, y| {
        polygon_contains([x, y], points)
    });
}

/// Circles are `[x, y, radius]`, merged with a smooth union of width `smooth`.
pub fn fill_blob(plot: &mut impl Plot, circles: &[[f64; 3]], smooth: f64) {
    let circles = &circles[..circles.len().min(MAX_BLOB_CIRCLES)];
    let Some(margin) = circles.iter().map(|c| c[2]).reduce(f64::max) else {
        return;
    };
    let mut centers = [[0.0; 2]; MAX_BLOB_CIRCLES];
    for (center, circle) in centers.iter_mut().zip(circles) {
        *center = [circle[0], circle[1]];
    }
    let bbox = bounding(&centers[..circles.len()], margin + smooth);
    fill_sdf(plot, bbox, |x, y| {
        circles
            .iter()
            .map(|c| libm::hypot(x - c[0], y - c[1]) - c[2])
            .reduce(|a, b| smooth_min(a, b, smooth))
            .unwrap_or(f64::MAX)
    });
}

fn smooth_min(a: f64, b: f64, k: f64) -> f64 {
    if k <= 0.0 {
        return a.min(b);
    }
    let h = (k - libm::fabs(a - b)).max(0.0) / k;
    a.min(b) - h * h * k * 0.25
}

/// Exactly `hypot(offset) - radius <= EDGE_TOLERANCE`; `hypot` only runs near the edge.
fn within(offset: [f64; 2], radius: f64) -> bool {
    let reach = radius + EDGE_TOLERANCE;
    let squared = offset[0] * offset[0] + offset[1] * offset[1];
    // A negative radius leaves `reach` tiny next to the rounding of `radius + EDGE_TOLERANCE`.
    if radius >= 0.0 && squared.is_finite() && (reach * reach).is_finite() {
        let limit = reach * reach;
        if squared < limit * (1.0 - SQUARED_MARGIN) {
            return true;
        }
        if squared > limit * (1.0 + SQUARED_MARGIN) {
            return false;
        }
    }
    libm::hypot(offset[0], offset[1]) - radius <= EDGE_TOLERANCE
}

/// From the nearest point of segment `ab` to `p`.
fn segment_offset(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    let (pa, ba) = ([p[0] - a[0], p[1] - a[1]], [b[0] - a[0], b[1] - a[1]]);
    let length = ba[0] * ba[0] + ba[1] * ba[1];
    let h = if length > 0.0 {
        ((pa[0] * ba[0] + pa[1] * ba[1]) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    [pa[0] - ba[0] * h, pa[1] - ba[1] * h]
}

/// Inside by the even-odd rule, or within `EDGE_TOLERANCE` of an edge: the signed distance
/// test without computing the distance of inside points.
fn polygon_contains(p: [f64; 2], points: &[[f64; 2]]) -> bool {
    let mut inside = false;
    let mut previous = points[points.len() - 1];
    for &point in points {
        let crosses = (point[1] > p[1]) != (previous[1] > p[1]);
        if crosses
            && p[0]
                < (previous[0] - point[0]) * (p[1] - point[1]) / (previous[1] - point[1]) + point[0]
        {
            inside = !inside;
        }
        previous = point;
    }
    inside
        || points
            .iter()
            .zip(points.iter().cycle().skip(points.len() - 1))
            .any(|(&point, &previous)| within(segment_offset(p, previous, point), 0.0))
}

fn bounding(points: &[[f64; 2]], margin: f64) -> Rect {
    let limit = f64::from(COORD_LIMIT);
    let (mut lo, mut hi) = ([limit, limit], [-limit, -limit]);
    for point in points {
        for axis in 0..2 {
            lo[axis] = lo[axis].min(point[axis] - margin);
            hi[axis] = hi[axis].max(point[axis] + margin);
        }
    }
    let x = clamp_floor(lo[0]);
    let y = clamp_floor(lo[1]);
    Rect {
        x,
        y,
        width: clamp_floor(hi[0]) + 2 - x,
        height: clamp_floor(hi[1]) + 2 - y,
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "clamped into ±COORD_LIMIT first"
)]
fn clamp_floor(value: f64) -> i32 {
    let limit = f64::from(COORD_LIMIT);
    px_core::floor(if value.is_nan() {
        0.0
    } else {
        value.clamp(-limit, limit)
    }) as i32
}
