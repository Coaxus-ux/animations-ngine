use px_core::{Palette, RampId, dither_keep};
use px_raster::Rect;

use super::composite::shifted;
use super::surface::{EMISSIVE, Surface};

pub(crate) struct Mirror {
    pub(crate) rect: Rect,
    pub(crate) axis: f64,
    pub(crate) ripple: f64,
    pub(crate) phase: f64,
    pub(crate) tone_shift: f64,
    pub(crate) opacity: f64,
    pub(crate) surface: Option<RampId>,
}

/// Copies the scene above `axis` into the region below it, upside down, with a ripple.
pub(crate) fn reflect(surface: &mut Surface<'_>, palette: &Palette, mirror: &Mirror) {
    let Rect {
        x: left,
        y: top,
        width,
        height,
    } = mirror.rect;
    #[expect(clippy::cast_possible_truncation, reason = "axis is a canvas row")]
    let axis = libm::round(mirror.axis) as i32;
    for y in top.max(axis)..(top + height).min(surface.height) {
        #[expect(clippy::cast_possible_truncation, reason = "ripple is a few pixels")]
        let shift =
            libm::round(mirror.ripple * libm::sin(f64::from(y) * 0.8 + mirror.phase)) as i32;
        for x in left.max(0)..(left + width).min(surface.width) {
            let (Some(target), Some(source)) = (
                surface.index(x, y),
                surface.index(x + shift, 2 * axis - y - 1),
            ) else {
                continue;
            };
            let on_surface = mirror
                .surface
                .is_none_or(|ramp| surface.cells[target].ramp == ramp);
            if !on_surface || !dither_keep(mirror.opacity, x, y) {
                continue;
            }
            let paint = surface.cells[source];
            let emissive = surface.flags[source] & EMISSIVE;
            let ramp_len = palette.ramp(paint.ramp).map_or(1, px_core::Ramp::len);
            let tone_shift = if emissive != 0 {
                0.0
            } else {
                mirror.tone_shift
            };
            surface.cells[target] = px_core::Paint {
                ramp: paint.ramp,
                tone: shifted(paint.tone, tone_shift, ramp_len, x, y),
            };
            surface.flags[target] = (surface.flags[target] & !EMISSIVE) | emissive;
        }
    }
}
