use px_core::{Paint, Palette, Tone, ToneName, dither_keep, dither_steps};
use px_raster::{Plot, Rect, fill_ellipse};

use super::mask::{FULL, Mask};
use super::surface::{CASTS_SHADOW, EMISSIVE, Region, Surface};
use crate::model::{Gradient, GradientDirection, Outline, Shading, Shape, Texture};

/// Everything about one shape that was evaluated for this frame.
pub(crate) struct Evaluated<'c> {
    pub(crate) depth: u16,
    pub(crate) seed: u64,
    pub(crate) opacity: f64,
    pub(crate) tone_shift: f64,
    pub(crate) contact_shadow: f64,
    pub(crate) shade_steps: f64,
    pub(crate) fog: f64,
    pub(crate) fog_paint: Option<Paint>,
    pub(crate) clips: &'c [Region],
    pub(crate) light: [f64; 3],
}

pub(crate) fn composite(
    shape: &Shape,
    frame: &Evaluated<'_>,
    palette: &Palette,
    mask: &mut Mask<'_>,
    surface: &mut Surface<'_>,
) {
    let style = &shape.style;
    if style.cleanup {
        mask.clean();
    }
    if frame.contact_shadow > 0.0 {
        contact_shadow(frame, palette, mask, surface);
    }
    if matches!(shape.geometry, crate::model::Geometry::Shade { .. }) {
        shade(frame, palette, mask, surface);
        return;
    }
    let max_distance = match style.shading {
        Shading::Dome { .. } => f64::from(mask.distance_field()),
        Shading::Flat => 1.0,
    };
    if let Some(texture) = style.texture {
        sample_texture(texture, frame.seed, mask);
    }
    let ramp_len = palette.ramp(shape.paint.ramp).map_or(1, px_core::Ramp::len);
    let flags = if style.emissive { EMISSIVE } else { 0 }
        | if style.casts_shadow { CASTS_SHADOW } else { 0 };
    let bbox = mask.bbox.unwrap_or_default();
    for (x, y) in mask.pixels() {
        let alpha = mask.alpha(x, y);
        let visible = alpha > 0
            && frame.clips.iter().all(|clip| clip.contains(x, y))
            && dither_keep(frame.opacity * f64::from(alpha) / f64::from(FULL), x, y);
        let Some(index) = surface.index(x, y).filter(|_| visible) else {
            continue;
        };
        let edge = EdgeContext {
            light: frame.light,
            paint: shape.paint,
            palette,
            max_distance,
        };
        let fill = Fill {
            shape,
            frame,
            bbox,
            max_distance,
            ramp_len,
        };
        let paint = outline_paint(style.outline, mask, x, y, &edge)
            .unwrap_or_else(|| fill.paint(mask, x, y));
        let paint = match frame.fog_paint {
            Some(fog) if frame.fog > 0.0 => fogged(paint, fog, frame.fog, palette),
            _ => paint,
        };
        surface.cells[index] = paint;
        surface.depth[index] = frame.depth;
        surface.flags[index] = flags;
    }
}

struct Fill<'a> {
    shape: &'a Shape,
    frame: &'a Evaluated<'a>,
    bbox: [i32; 4],
    max_distance: f64,
    ramp_len: u8,
}

impl Fill<'_> {
    fn paint(&self, mask: &Mask<'_>, x: i32, y: i32) -> Paint {
        let style = &self.shape.style;
        let mut offset = self.frame.tone_shift;
        if let Shading::Dome { steps, soft } = style.shading {
            offset += shading(mask, x, y, self.max_distance, self.frame.light, steps, soft);
        }
        if let Some(gradient) = style.gradient {
            offset += f64::from(dither_steps(
                gradient_value(gradient, self.bbox, x, y),
                x,
                y,
            ));
        }
        if style.texture.is_some() {
            offset += f64::from(clustered_noise(mask, x, y));
        }
        Paint {
            ramp: self.shape.paint.ramp,
            tone: shifted(self.shape.paint.tone, offset, self.ramp_len, x, y),
        }
    }
}

/// Atmospheric perspective: the tone keeps its place in the ramp, moved into the fog ramp
/// and pulled toward the fog tone. Rounded, not dithered: fog is uniform across a layer.
fn fogged(paint: Paint, fog: Paint, amount: f64, palette: &Palette) -> Paint {
    let length = |ramp| palette.ramp(ramp).map_or(1, px_core::Ramp::len);
    let (own, target) = (length(paint.ramp), length(fog.ramp));
    let place = if own > 1 {
        f64::from(paint.tone.index()) / f64::from(own - 1)
    } else {
        0.5
    };
    let start = place * f64::from(target.saturating_sub(1));
    let value = start + (f64::from(fog.tone.index()) - start) * amount.min(1.0);
    Paint {
        ramp: fog.ramp,
        tone: shifted(Tone::new(0), libm::round(value), target, 0, 0),
    }
}

pub(crate) fn shifted(tone: Tone, offset: f64, ramp_len: u8, x: i32, y: i32) -> Tone {
    let value = i32::from(tone.index()) + dither_steps(offset, x, y);
    Tone::new(u8::try_from(value.clamp(0, i32::from(ramp_len.max(1)) - 1)).unwrap_or(0))
}

#[derive(Clone, Copy)]
struct EdgeContext<'p> {
    light: [f64; 3],
    paint: Paint,
    palette: &'p Palette,
    max_distance: f64,
}

fn outline_paint(
    outline: Outline,
    mask: &Mask<'_>,
    x: i32,
    y: i32,
    edge: &EdgeContext<'_>,
) -> Option<Paint> {
    let EdgeContext {
        light,
        paint,
        palette,
        max_distance,
    } = *edge;
    let (Outline::Full(color) | Outline::Selective(color)) = outline else {
        return None;
    };
    if !mask.is_edge(x, y) || is_l_corner(mask, x, y) {
        return None;
    }
    if let Outline::Selective(_) = outline {
        let [nx, ny, _] = normal(mask, x, y, max_distance.max(3.0));
        if nx * light[0] + ny * light[1] > 0.2 {
            let shade = palette
                .ramp(paint.ramp)
                .ok()
                .and_then(|ramp| ramp.named(ToneName::Shade))
                .unwrap_or(paint.tone);
            return Some(Paint {
                ramp: paint.ramp,
                tone: shade,
            });
        }
    }
    Some(color)
}

// Pixel-perfect lines: an edge pixel that only turns a corner doubles the line (RENDERING §7.2).
fn is_l_corner(mask: &Mask<'_>, x: i32, y: i32) -> bool {
    let edge = |dx: i32, dy: i32| mask.is_edge(x + dx, y + dy);
    let neighbors = [
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ]
    .iter()
    .filter(|(dx, dy)| edge(*dx, *dy))
    .count();
    neighbors == 2
        && [(0, -1, 1, 0), (1, 0, 0, 1), (0, 1, -1, 0), (-1, 0, 0, -1)]
            .iter()
            .any(|&(ax, ay, bx, by)| edge(ax, ay) && edge(bx, by) && !edge(ax + bx, ay + by))
}

fn height(mask: &Mask<'_>, x: i32, y: i32, max_distance: f64) -> f64 {
    libm::sqrt((f64::from(mask.distance_at(x, y)) / max_distance).min(1.0))
}

fn normal(mask: &Mask<'_>, x: i32, y: i32, max_distance: f64) -> [f64; 3] {
    let tilt = max_distance / 3.0 * 0.6;
    let gx = height(mask, x + 1, y, max_distance) - height(mask, x - 1, y, max_distance);
    let gy = height(mask, x, y + 1, max_distance) - height(mask, x, y - 1, max_distance);
    let (nx, ny) = (-gx * tilt, -gy * tilt);
    let length = libm::sqrt(nx * nx + ny * ny + 1.0);
    [nx / length, ny / length, 1.0 / length]
}

fn shading(
    mask: &Mask<'_>,
    x: i32,
    y: i32,
    max_distance: f64,
    light: [f64; 3],
    steps: u8,
    soft: bool,
) -> f64 {
    let n = normal(mask, x, y, max_distance);
    let lit = n[0] * light[0] + n[1] * light[1] + n[2] * light[2] - light[2];
    let value = lit * 2.5 * f64::from(steps);
    // Dithered bands only read as gradients on larger forms; small ones get hard bands (cleanup spike).
    if soft && max_distance >= 24.0 {
        value
    } else {
        libm::round(value)
    }
}

fn gradient_value(gradient: Gradient, [x0, y0, x1, y1]: [i32; 4], x: i32, y: i32) -> f64 {
    let span =
        |from: i32, to: i32, at: i32| f64::from(at - from) / f64::from((to - from - 1).max(1));
    let k = match gradient.direction {
        GradientDirection::Down => span(y0, y1, y),
        GradientDirection::Up => 1.0 - span(y0, y1, y),
        GradientDirection::Right => span(x0, x1, x),
        GradientDirection::Left => 1.0 - span(x0, x1, x),
        GradientDirection::Radial => {
            let (cx, cy) = (f64::from(x0 + x1) / 2.0, f64::from(y0 + y1) / 2.0);
            let radius = libm::hypot(f64::from(x1 - x0), f64::from(y1 - y0)) / 2.0;
            libm::hypot(f64::from(x) + 0.5 - cx, f64::from(y) + 0.5 - cy) / radius.max(1.0)
        }
    };
    k.clamp(0.0, 1.0) * f64::from(gradient.steps)
}

fn sample_texture(texture: Texture, seed: u64, mask: &mut Mask<'_>) {
    let scale = texture.scale.max(1.0);
    for (x, y) in mask.pixels() {
        if let Some(index) = mask.index(x, y) {
            let n = px_expr::value_noise(seed, f64::from(x) / scale, f64::from(y) / scale);
            mask.noise[index] = if n > 1.0 - texture.amount / 2.0 {
                1
            } else if n < texture.amount / 2.0 {
                -1
            } else {
                0
            };
        }
    }
}

// A 3×3 majority vote groups noise into clumps instead of static (RENDERING §7.4).
fn clustered_noise(mask: &Mask<'_>, x: i32, y: i32) -> i8 {
    let raw = |dx: i32, dy: i32| {
        mask.index(x + dx, y + dy)
            .filter(|_| mask.covered(x + dx, y + dy))
            .map_or(0, |i| mask.noise[i])
    };
    let mut counts = [0u8; 3];
    for dy in -1..=1 {
        for dx in -1..=1 {
            counts[usize::try_from(raw(dx, dy) + 1).unwrap_or(1)] += 1;
        }
    }
    match counts.iter().position(|count| *count >= 5) {
        Some(winner) => i8::try_from(winner).unwrap_or(1) - 1,
        None => raw(0, 0),
    }
}

fn shade(frame: &Evaluated<'_>, palette: &Palette, mask: &Mask<'_>, surface: &mut Surface<'_>) {
    for (x, y) in mask.pixels() {
        if !mask.covered(x, y) || !frame.clips.iter().all(|clip| clip.contains(x, y)) {
            continue;
        }
        darken(surface, palette, x, y, frame.shade_steps);
    }
}

pub(crate) fn darken(surface: &mut Surface<'_>, palette: &Palette, x: i32, y: i32, steps: f64) {
    let Some(index) = surface.index(x, y) else {
        return;
    };
    if surface.flags[index] & EMISSIVE != 0 {
        return;
    }
    let paint = surface.cells[index];
    let ramp_len = palette.ramp(paint.ramp).map_or(1, px_core::Ramp::len);
    surface.cells[index] = Paint {
        ramp: paint.ramp,
        tone: shifted(paint.tone, steps, ramp_len, x, y),
    };
}

struct Collect<'s, 'a> {
    surface: &'s mut Surface<'a>,
    palette: &'s Palette,
    steps: f64,
}

impl Plot for Collect<'_, '_> {
    fn width(&self) -> i32 {
        self.surface.width
    }

    fn height(&self) -> i32 {
        self.surface.height
    }

    fn span(&mut self, y: i32, x_start: i32, x_end: i32) {
        for x in x_start.max(0)..x_end.min(self.surface.width) {
            darken(self.surface, self.palette, x, y, -self.steps);
        }
    }
}

fn contact_shadow(
    frame: &Evaluated<'_>,
    palette: &Palette,
    mask: &Mask<'_>,
    surface: &mut Surface<'_>,
) {
    let Some([x0, _, x1, y1]) = mask.bbox else {
        return;
    };
    let width = x1 - x0 + 2;
    let height = (width / 4).clamp(2, 6);
    let rect = Rect {
        x: x0 - 1,
        y: y1 - height / 2,
        width,
        height,
    };
    fill_ellipse(
        &mut Collect {
            surface,
            palette,
            steps: frame.contact_shadow,
        },
        rect,
    );
}
