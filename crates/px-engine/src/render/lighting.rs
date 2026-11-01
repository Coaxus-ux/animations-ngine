use px_core::{LightColor, Paint, Palette};
use px_raster::Rect;

use super::composite::shifted;
use super::shadow::{Memo, Shadows};
use super::surface::{EMISSIVE, Surface};
use crate::model::MAX_LIGHTS;

const CONE_SOFTNESS: f64 = 0.45;
const CONE_EDGE: f64 = 0.12;
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

pub(crate) struct Scratch<'a> {
    pub(crate) window: &'a mut [u16],
    pub(crate) memo: &'a mut [Memo],
}

pub(crate) struct Ambient {
    pub(crate) darkness: f64,
    pub(crate) occlusion: f64,
}

pub(crate) fn light(
    surface: &mut Surface<'_>,
    palette: &Palette,
    ambient: &Ambient,
    lights: &[Lit],
    scratch: Scratch<'_>,
    grain: i32,
) {
    let Scratch { window, memo } = scratch;
    if ambient.darkness == 0.0 && ambient.occlusion == 0.0 && lights.is_empty() {
        return;
    }
    if ambient.occlusion > 0.0 {
        nearest_in_rows(surface, window);
    }
    let casting = lights.iter().any(|lamp| lamp.shadows);
    let mut shadows = Shadows::new(surface, casting, memo, grain);
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
            for (light, ((lamp, reach), in_row)) in
                lights.iter().zip(&boxes).zip(lit_row).enumerate()
            {
                if !in_row || reach.misses_column(center_x) {
                    continue;
                }
                let amount = contribution(lamp, x, y);
                if amount <= 0.0
                    || (lamp.shadows
                        && shadows.blocked(
                            surface,
                            light,
                            (x, y),
                            source_point(lamp, center_x, f64::from(y) + 0.5),
                        ))
                {
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
