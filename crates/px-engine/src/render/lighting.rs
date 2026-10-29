use px_core::{LightColor, Paint, Palette};
use px_raster::Rect;

use super::composite::shifted;
use super::surface::{CASTS_SHADOW, EMISSIVE, Surface};
use crate::model::MAX_LIGHTS;

const MAX_SHADOW_STEPS: f64 = 160.0;
/// The outer part of a cone's half spread that fades to dark (with a floor, in radians).
const CONE_SOFTNESS: f64 = 0.45;
const CONE_EDGE: f64 = 0.12;
/// Squared distances beyond `reach² × REACH_MARGIN` are out of reach whatever `hypot` rounds
/// to, so they skip it; the margin dwarfs float error and changes no result.
const REACH_MARGIN: f64 = 1.0 + 1e-9;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Shape {
    Point,
    Cone { direction: f64, half_spread: f64 },
    Rect(Rect),
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Lit {
    pub(crate) shape: Shape,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) reach: f64,
    pub(crate) intensity: f64,
    pub(crate) color: Option<LightColor>,
    pub(crate) shadows: bool,
}

pub(crate) struct Ambient {
    pub(crate) darkness: f64,
    pub(crate) occlusion: f64,
}

/// `window` is scratch with one value per pixel.
pub(crate) fn light(
    surface: &mut Surface<'_>,
    palette: &Palette,
    ambient: &Ambient,
    lights: &[Lit],
    window: &mut [u16],
) {
    if ambient.darkness == 0.0 && ambient.occlusion == 0.0 && lights.is_empty() {
        return;
    }
    if ambient.occlusion > 0.0 {
        nearest_in_rows(surface, window);
    }
    let reaches = lights.iter().map(Reach::of);
    let mut boxes = [Reach::EVERYWHERE; MAX_LIGHTS];
    for (target, reach) in boxes.iter_mut().zip(reaches) {
        *target = reach;
    }
    for y in 0..surface.height {
        let center_y = f64::from(y) + 0.5;
        let rows = lights
            .iter()
            .zip(&boxes)
            .map(|(lamp, reach)| (lamp, reach, reach.has_row(center_y)));
        let mut lit_row = [false; MAX_LIGHTS];
        for (slot, (_, _, has)) in lit_row.iter_mut().zip(rows) {
            *slot = has;
        }
        for x in 0..surface.width {
            let Some(index) = surface.index(x, y) else {
                continue;
            };
            if surface.flags[index] & EMISSIVE != 0 {
                continue;
            }
            let mut steps = -ambient.darkness;
            if ambient.occlusion > 0.0 {
                steps -= ambient.occlusion * occluded(surface, window, x, y);
            }
            let center_x = f64::from(x) + 0.5;
            let mut strongest: (f64, Option<LightColor>) = (0.0, None);
            for ((lamp, reach), in_row) in lights.iter().zip(&boxes).zip(lit_row) {
                if !in_row || reach.misses_column(center_x) {
                    continue;
                }
                let amount = contribution(lamp, x, y);
                if amount <= 0.0 || (lamp.shadows && shadowed(surface, x, y, lamp)) {
                    continue;
                }
                steps += amount;
                if amount > strongest.0 {
                    strongest = (amount, lamp.color);
                }
            }
            let paint = surface.cells[index];
            let ramp = match strongest {
                (amount, Some(color)) if amount >= 1.0 => {
                    palette.twin(paint.ramp, color).unwrap_or(paint.ramp)
                }
                _ => paint.ramp,
            };
            let ramp_len = palette.ramp(ramp).map_or(1, px_core::Ramp::len);
            surface.cells[index] = Paint {
                ramp,
                tone: shifted(paint.tone, steps, ramp_len, x, y),
            };
        }
    }
}

/// The pixel centers a light can reach, padded by a pixel: outside it `contribution` is
/// exactly 0. Bounds that aren't finite never exclude anything, so non-finite lights keep
/// their exact (NaN) behavior.
#[derive(Clone, Copy, Debug)]
struct Reach {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

impl Reach {
    const EVERYWHERE: Self = Self {
        left: f64::NEG_INFINITY,
        right: f64::INFINITY,
        top: f64::NEG_INFINITY,
        bottom: f64::INFINITY,
    };

    fn of(lamp: &Lit) -> Self {
        // Relative slack too, so even huge reaches stay wider than `REACH_MARGIN` allows.
        let pad = lamp.reach * (1.0 + 1e-6) + 1.0;
        let (left, top, right, bottom) = match lamp.shape {
            Shape::Rect(rect) => (
                f64::from(rect.x),
                f64::from(rect.y),
                f64::from(rect.x + rect.width),
                f64::from(rect.y + rect.height),
            ),
            Shape::Point | Shape::Cone { .. } => (lamp.x, lamp.y, lamp.x, lamp.y),
        };
        Self {
            left: left - pad,
            right: right + pad,
            top: top - pad,
            bottom: bottom + pad,
        }
    }

    fn has_row(&self, y: f64) -> bool {
        !(y < self.top || y > self.bottom)
    }

    fn misses_column(&self, x: f64) -> bool {
        x < self.left || x > self.right
    }
}

/// The deepest (nearest) depth within two columns of each pixel, per row: five of these
/// rows bound a 5×5 window, so most pixels skip counting their neighbors.
fn nearest_in_rows(surface: &Surface<'_>, window: &mut [u16]) {
    let width = usize::try_from(surface.width).unwrap_or(0);
    for (row, out) in surface
        .depth
        .chunks_exact(width.max(1))
        .zip(window.chunks_exact_mut(width.max(1)))
    {
        for (x, nearest) in out.iter_mut().enumerate() {
            let start = x.saturating_sub(2);
            let end = (x + 3).min(width);
            *nearest = row[start..end].iter().copied().max().unwrap_or(0);
        }
    }
}

fn contribution(lamp: &Lit, x: i32, y: i32) -> f64 {
    let (px, py) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
    let (target_x, target_y) = source_point(lamp, px, py);
    let (dx, dy) = (px - target_x, py - target_y);
    if lamp.reach <= 0.0 || dx * dx + dy * dy > lamp.reach * lamp.reach * REACH_MARGIN {
        return 0.0;
    }
    let distance = libm::hypot(dx, dy);
    if distance >= lamp.reach {
        return 0.0;
    }
    let falloff = 1.0 - distance / lamp.reach;
    let aim = match lamp.shape {
        Shape::Cone {
            direction,
            half_spread,
        } => {
            let angle = libm::atan2(py - lamp.y, px - lamp.x);
            let off = libm::fabs(libm::remainder(angle - direction, core::f64::consts::TAU));
            let edge = (half_spread * CONE_SOFTNESS).max(CONE_EDGE);
            ((half_spread - off) / edge).clamp(0.0, 1.0)
        }
        _ => 1.0,
    };
    lamp.intensity * falloff * aim
}

fn source_point(lamp: &Lit, px: f64, py: f64) -> (f64, f64) {
    match lamp.shape {
        Shape::Rect(rect) => (
            px.clamp(f64::from(rect.x), f64::from(rect.x + rect.width)),
            py.clamp(f64::from(rect.y), f64::from(rect.y + rect.height)),
        ),
        _ => (lamp.x, lamp.y),
    }
}

/// A pixel is in shadow when a shadow-casting pixel in front of it lies between it and the light.
fn shadowed(surface: &Surface<'_>, x: i32, y: i32, lamp: &Lit) -> bool {
    let Some(own) = surface.index(x, y).map(|index| surface.depth[index]) else {
        return false;
    };
    let (px, py) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
    let (lx, ly) = source_point(lamp, px, py);
    let distance = libm::hypot(lx - px, ly - py).min(MAX_SHADOW_STEPS);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "bounded by MAX_SHADOW_STEPS"
    )]
    let steps = distance as u32;
    (1..steps).any(|step| {
        let t = f64::from(step) / f64::from(steps);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "on-canvas sample positions fit i32"
        )]
        let (sx, sy) = (
            px_core::floor(px + (lx - px) * t) as i32,
            px_core::floor(py + (ly - py) * t) as i32,
        );
        surface
            .index(sx, sy)
            .is_some_and(|i| surface.flags[i] & CASTS_SHADOW != 0 && surface.depth[i] > own)
    })
}

fn occluded(surface: &Surface<'_>, window: &[u16], x: i32, y: i32) -> f64 {
    let Some(own) = surface.index(x, y).map(|index| surface.depth[index]) else {
        return 0.0;
    };
    let (top, bottom) = ((y - 2).max(0), (y + 2).min(surface.height - 1));
    let nearest = (top..=bottom)
        .filter_map(|row| surface.index(x, row))
        .map(|index| window[index])
        .max()
        .unwrap_or(0);
    if nearest <= own {
        return 0.0;
    }
    let (left, right) = ((x - 2).max(0), (x + 2).min(surface.width - 1));
    let mut closer = 0;
    for row in top..=bottom {
        if let (Some(start), Some(end)) = (surface.index(left, row), surface.index(right, row)) {
            closer += surface.depth[start..=end]
                .iter()
                .filter(|depth| **depth > own)
                .count();
        }
    }
    let closer = f64::from(u32::try_from(closer).unwrap_or(u32::MAX));
    (closer / 24.0 * 2.0).min(1.0)
}
