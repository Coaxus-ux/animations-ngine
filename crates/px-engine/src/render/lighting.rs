use px_core::{LightColor, Paint, Palette};
use px_raster::Rect;

use super::composite::shifted;
use super::surface::{CASTS_SHADOW, EMISSIVE, Surface};

const MAX_SHADOW_STEPS: f64 = 160.0;
/// The outer part of a cone's half spread that fades to dark (with a floor, in radians).
const CONE_SOFTNESS: f64 = 0.45;
const CONE_EDGE: f64 = 0.12;

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

pub(crate) fn light(
    surface: &mut Surface<'_>,
    palette: &Palette,
    ambient: &Ambient,
    lights: &[Lit],
) {
    if ambient.darkness == 0.0 && ambient.occlusion == 0.0 && lights.is_empty() {
        return;
    }
    for y in 0..surface.height {
        for x in 0..surface.width {
            let Some(index) = surface.index(x, y) else {
                continue;
            };
            if surface.flags[index] & EMISSIVE != 0 {
                continue;
            }
            let mut steps = -ambient.darkness;
            if ambient.occlusion > 0.0 {
                steps -= ambient.occlusion * occluded(surface, x, y);
            }
            let mut strongest: (f64, Option<LightColor>) = (0.0, None);
            for lamp in lights {
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

fn contribution(lamp: &Lit, x: i32, y: i32) -> f64 {
    let (px, py) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
    let (target_x, target_y) = source_point(lamp, px, py);
    let distance = libm::hypot(px - target_x, py - target_y);
    if lamp.reach <= 0.0 || distance >= lamp.reach {
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
            libm::floor(px + (lx - px) * t) as i32,
            libm::floor(py + (ly - py) * t) as i32,
        );
        surface
            .index(sx, sy)
            .is_some_and(|i| surface.flags[i] & CASTS_SHADOW != 0 && surface.depth[i] > own)
    })
}

fn occluded(surface: &Surface<'_>, x: i32, y: i32) -> f64 {
    let Some(own) = surface.index(x, y).map(|index| surface.depth[index]) else {
        return 0.0;
    };
    let mut closer = 0u8;
    for dy in -2..=2 {
        for dx in -2..=2 {
            if surface
                .index(x + dx, y + dy)
                .is_some_and(|i| surface.depth[i] > own)
            {
                closer += 1;
            }
        }
    }
    (f64::from(closer) / 24.0 * 2.0).min(1.0)
}
