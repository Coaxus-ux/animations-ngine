use core::f64::consts::PI;

use px_core::{hash_seed, snap_coord, snap_extent};
use px_raster::{
    MAX_BLOB_CIRCLES, Point, Rect, draw_line, fill_blob, fill_capsule, fill_ellipse, fill_polygon,
    fill_rect, fill_rounded_rect,
};

use super::RenderError;
use super::mask::{FULL, Mask};
use super::values::Values;
use crate::model::{Bounds, Geometry, Particles, Segment};

const ROPE_SEGMENTS: usize = 16;

pub(crate) fn rasterize(
    geometry: &Geometry,
    values: &mut Values<'_>,
    mask: &mut Mask<'_>,
) -> Result<(), RenderError> {
    match geometry {
        Geometry::Rect(bounds)
        | Geometry::Shade {
            bounds,
            round: false,
            ..
        } => fill_rect(mask, rect(bounds, values)?),
        Geometry::Ellipse(bounds)
        | Geometry::Shade {
            bounds,
            round: true,
            ..
        } => fill_ellipse(mask, rect(bounds, values)?),
        Geometry::RoundedRect { bounds, radius } => {
            let rect = rect(bounds, values)?;
            fill_rounded_rect(mask, rect, values.of(radius)?);
        }
        Geometry::Line { segment, width } => {
            let (from, to) = ends(segment, values)?;
            stroke(mask, from, to, values.of(width)?);
        }
        Geometry::Polygon { x, y, points } => {
            let (dx, dy) = (values.x(x)?, values.y(y)?);
            let mut moved = [[0.0; 2]; px_raster::MAX_POLYGON_POINTS];
            for (target, point) in moved.iter_mut().zip(points) {
                *target = [point[0] + dx, point[1] + dy];
            }
            fill_polygon(mask, &moved[..points.len().min(moved.len())]);
        }
        Geometry::Blob {
            x,
            y,
            circles,
            smooth,
        } => {
            let (dx, dy) = (values.x(x)?, values.y(y)?);
            let mut moved = [[0.0; 3]; MAX_BLOB_CIRCLES];
            for (target, circle) in moved.iter_mut().zip(circles) {
                *target = [circle[0] + dx, circle[1] + dy, circle[2]];
            }
            fill_blob(
                mask,
                &moved[..circles.len().min(MAX_BLOB_CIRCLES)],
                values.of(smooth)?,
            );
        }
        Geometry::Rope {
            segment,
            sag,
            sway,
            width,
        } => rope(
            segment,
            [values.of(sag)?, values.of(sway)?, values.of(width)?],
            values,
            mask,
        )?,
        Geometry::Particles(particles) => self::particles(particles, values, mask)?,
    }
    Ok(())
}

fn rect(bounds: &Bounds, values: &mut Values<'_>) -> Result<Rect, RenderError> {
    Ok(Rect {
        x: snap_coord(values.x(&bounds.x)?),
        y: snap_coord(values.y(&bounds.y)?),
        width: snap_extent(values.of(&bounds.width)?),
        height: snap_extent(values.of(&bounds.height)?),
    })
}

fn ends(segment: &Segment, values: &mut Values<'_>) -> Result<([f64; 2], [f64; 2]), RenderError> {
    Ok((
        [values.x(&segment.x1)?, values.y(&segment.y1)?],
        [values.x(&segment.x2)?, values.y(&segment.y2)?],
    ))
}

fn stroke(mask: &mut Mask<'_>, from: [f64; 2], to: [f64; 2], width: f64) {
    if width <= 1.5 {
        let point = |p: [f64; 2]| Point {
            x: snap_coord(p[0]),
            y: snap_coord(p[1]),
        };
        draw_line(mask, point(from), point(to));
    } else {
        fill_capsule(mask, from, to, width / 2.0);
    }
}

fn rope(
    segment: &Segment,
    [sag, sway, width]: [f64; 3],
    values: &mut Values<'_>,
    mask: &mut Mask<'_>,
) -> Result<(), RenderError> {
    let (from, to) = ends(segment, values)?;
    let at = |step: usize| {
        let s = f64::from(u8::try_from(step).unwrap_or(0))
            / f64::from(u8::try_from(ROPE_SEGMENTS).unwrap_or(1));
        let bulge = libm::sin(PI * s);
        [
            from[0] + (to[0] - from[0]) * s + sway * bulge,
            from[1] + (to[1] - from[1]) * s + sag * 4.0 * s * (1.0 - s),
        ]
    };
    for step in 0..ROPE_SEGMENTS {
        stroke(mask, at(step), at(step + 1), width);
    }
    Ok(())
}

#[expect(
    clippy::cast_precision_loss,
    reason = "53 high bits fit an f64 mantissa exactly"
)]
fn unit(bits: u64) -> f64 {
    (bits >> 11) as f64 / (1u64 << 53) as f64
}

fn particles(
    spec: &Particles,
    values: &mut Values<'_>,
    mask: &mut Mask<'_>,
) -> Result<(), RenderError> {
    let [x, y, width, height] = [
        values.x(&spec.area.x)?,
        values.y(&spec.area.y)?,
        values.of(&spec.area.width)?,
        values.of(&spec.area.height)?,
    ];
    let (vx, vy) = (values.of(&spec.velocity_x)?, values.of(&spec.velocity_y)?);
    let (gravity, life) = (values.of(&spec.gravity)?, values.of(&spec.life)?);
    let (size, wind_factor) = (values.of(&spec.size)?, values.of(&spec.wind)?);
    let life = life.max(1.0 / values.env.fps.max(1.0));
    let dot = snap_extent(size).max(1);
    for index in 0..u64::from(spec.count) {
        let random = |key: u64| unit(hash_seed(values.env.seed, &[index, key]));
        let age = (values.env.time + random(2) * life).rem_euclid(life);
        let spawn = [x + random(0) * width, y + random(1) * height];
        let wind = wind_factor
            * px_expr::wind_at(
                values.env.wind,
                values.env.time,
                spawn[0],
                values.env.loop_period,
            );
        let px = spawn[0] + (vx + wind) * age;
        let py = spawn[1] + vy * age + 0.5 * gravity * age * age;
        let fade = if spec.fade { 1.0 - age / life } else { 1.0 };
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to 1..=255"
        )]
        let alpha = (fade.clamp(0.0, 1.0) * f64::from(FULL)).round().max(1.0) as u8;
        let (left, top) = (snap_coord(px), snap_coord(py));
        for dy in 0..dot {
            for dx in 0..dot {
                mask.set_alpha(left + dx, top + dy, alpha);
            }
        }
    }
    Ok(())
}
