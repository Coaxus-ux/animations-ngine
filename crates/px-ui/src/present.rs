use px_core::{Palette, Rgb};

use crate::canvas::{Overlay, UiCanvas};

/// `0x00RRGGBB`, the pixel format window buffers take.
#[must_use]
pub const fn pack(color: Rgb) -> u32 {
    (color.r as u32) << 16 | (color.g as u32) << 8 | color.b as u32
}

/// Writes the canvas, `scale` window pixels per UI pixel, into `out` (`width × height`
/// window pixels, row by row), then its exact-color overlays.
pub fn present(canvas: &UiCanvas, palette: &Palette, scale: i32, out: &mut [u32], width: usize) {
    let scale = scale.max(1);
    let columns = usize::try_from(canvas.width()).unwrap_or(0);
    let factor = usize::try_from(scale).unwrap_or(1);
    for (row, cells) in canvas.cells().chunks(columns.max(1)).enumerate() {
        for (column, paint) in cells.iter().enumerate() {
            let color = pack(palette.resolve(*paint).unwrap_or_default());
            fill(
                out,
                width,
                column * factor,
                row * factor,
                factor,
                factor,
                color,
            );
        }
    }
    present_overlays(canvas.overlays(), scale, out, width);
}

/// Fills a box of window pixels, clipped to the buffer.
pub fn fill(out: &mut [u32], width: usize, x: usize, y: usize, w: usize, h: usize, color: u32) {
    if width == 0 {
        return;
    }
    let height = out.len() / width;
    for row in y..(y + h).min(height) {
        let start = row * width + x.min(width);
        let end = row * width + (x + w).min(width);
        out[start..end].fill(color);
    }
}

/// Writes the canvas's top layer over `out` (after the document is drawn there).
pub fn present_top(
    canvas: &UiCanvas,
    palette: &Palette,
    scale: i32,
    out: &mut [u32],
    width: usize,
) {
    let scale = scale.max(1);
    let columns = usize::try_from(canvas.width()).unwrap_or(0);
    let factor = usize::try_from(scale).unwrap_or(1);
    for (row, cells) in canvas.top().chunks(columns.max(1)).enumerate() {
        for (column, paint) in cells.iter().enumerate() {
            if let Some(paint) = paint {
                let color = pack(palette.resolve(*paint).unwrap_or_default());
                fill(
                    out,
                    width,
                    column * factor,
                    row * factor,
                    factor,
                    factor,
                    color,
                );
            }
        }
    }
    present_overlays(canvas.top_overlays(), scale, out, width);
}

fn present_overlays(overlays: &[Overlay], scale: i32, out: &mut [u32], width: usize) {
    let to = |value: i32| usize::try_from(value * scale).unwrap_or(0);
    let factor = usize::try_from(scale).unwrap_or(1);
    for overlay in overlays {
        match overlay {
            Overlay::Box(area, color) => {
                let (w, h) = (to(area.width), to(area.height));
                fill(out, width, to(area.x), to(area.y), w, h, pack(*color));
            }
            Overlay::Image(area, pixels) => {
                let columns = usize::try_from(area.width).unwrap_or(1).max(1);
                for (row, line) in pixels.chunks(columns).enumerate() {
                    for (column, color) in line.iter().enumerate() {
                        let x = to(area.x) + column * factor;
                        let y = to(area.y) + row * factor;
                        fill(out, width, x, y, factor, factor, pack(*color));
                    }
                }
            }
        }
    }
}
