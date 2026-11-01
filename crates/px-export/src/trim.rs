use crate::rgba::RgbaImage;
use crate::sheet::CellRect;

#[must_use]
pub fn trim_bounds(frames: &[RgbaImage]) -> Option<CellRect> {
    let bounds = opaque_bounds(frames)?;
    let whole = frames.first().is_some_and(|first| {
        bounds.x == 0 && bounds.y == 0 && bounds.w == first.width && bounds.h == first.height
    });
    (!whole).then_some(bounds)
}

fn opaque_bounds(frames: &[RgbaImage]) -> Option<CellRect> {
    let mut bounds: Option<(u32, u32, u32, u32)> = None;
    for image in frames {
        let Some(columns) = usize::try_from(image.width).ok().filter(|width| *width > 0) else {
            continue;
        };
        for (y, row) in image.pixels.chunks_exact(columns).enumerate() {
            let Some(first) = row.iter().position(Option::is_some) else {
                continue;
            };
            let last = row.iter().rposition(Option::is_some).unwrap_or(first);
            let (y, first, last) = (narrow(y), narrow(first), narrow(last));
            bounds = Some(bounds.map_or((first, y, last, y), |(x0, y0, x1, y1)| {
                (x0.min(first), y0.min(y), x1.max(last), y1.max(y))
            }));
        }
    }
    bounds.map(|(x0, y0, x1, y1)| CellRect {
        x: x0,
        y: y0,
        w: x1 - x0 + 1,
        h: y1 - y0 + 1,
    })
}

#[must_use]
pub fn crop(image: &RgbaImage, bounds: CellRect) -> RgbaImage {
    let count = usize::try_from(u64::from(bounds.w) * u64::from(bounds.h)).unwrap_or(0);
    let mut pixels = vec![None; count];
    let (columns, cropped) = (
        usize::try_from(image.width).unwrap_or(0),
        usize::try_from(bounds.w).unwrap_or(0),
    );
    let span = image.width.saturating_sub(bounds.x).min(bounds.w);
    let rows = image.height.saturating_sub(bounds.y).min(bounds.h);
    for row in 0..usize::try_from(rows).unwrap_or(0) {
        let from = (row + usize::try_from(bounds.y).unwrap_or(0)) * columns
            + usize::try_from(bounds.x).unwrap_or(0);
        let width = usize::try_from(span).unwrap_or(0);
        pixels[row * cropped..row * cropped + width]
            .copy_from_slice(&image.pixels[from..from + width]);
    }
    RgbaImage {
        width: bounds.w,
        height: bounds.h,
        pixels,
    }
}

fn narrow(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
