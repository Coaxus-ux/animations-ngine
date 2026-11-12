use px_core::{Paint, Palette};
use px_ui::{Rect, Roles, fill, pack};

use super::render_cache::RenderCache;
use crate::app::ViewState;
use crate::domain::Selection;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Placement {
    pub zoom: i32,
    pub origin: (i32, i32),
}

impl Placement {
    #[must_use]
    pub fn of(view: &ViewState, area: Rect, canvas: (i32, i32)) -> Self {
        let fit = (area.width / canvas.0.max(1))
            .min(area.height / canvas.1.max(1))
            .max(1);
        let zoom = if view.zoom > 0 { view.zoom } else { fit };
        let centered = (
            (area.width - canvas.0 * zoom) / 2,
            (area.height - canvas.1 * zoom) / 2,
        );
        let (dx, dy) = view.pan.unwrap_or(centered);
        Self {
            zoom,
            origin: (area.x + dx, area.y + dy),
        }
    }

    #[must_use]
    pub const fn to_canvas(self, x: i32, y: i32) -> (i32, i32) {
        (
            (x - self.origin.0).div_euclid(self.zoom),
            (y - self.origin.1).div_euclid(self.zoom),
        )
    }
}

pub struct ViewPixels<'a> {
    pub out: &'a mut [u32],
    pub width: usize,
    pub clip: Rect,
}

impl ViewPixels<'_> {
    pub fn block(&mut self, placement: Placement, (x, y): (i32, i32), color: u32, dithered: bool) {
        let z = placement.zoom;
        let left = placement.origin.0 + x * z;
        let top = placement.origin.1 + y * z;
        for wy in top.max(self.clip.y)..(top + z).min(self.clip.y + self.clip.height) {
            for wx in left.max(self.clip.x)..(left + z).min(self.clip.x + self.clip.width) {
                if !dithered || (wx + wy) % 2 == 0 {
                    self.set(wx, wy, color);
                }
            }
        }
    }

    pub fn set(&mut self, x: i32, y: i32, color: u32) {
        if let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y))
            && x < self.width
            && let Some(pixel) = self.out.get_mut(y * self.width + x)
        {
            *pixel = color;
        }
    }

    pub fn rect(&mut self, area: Rect, color: u32) {
        let area = clip(area, self.clip);
        let to = |value: i32| usize::try_from(value).unwrap_or(0);
        fill(
            self.out,
            self.width,
            to(area.x),
            to(area.y),
            to(area.width),
            to(area.height),
            color,
        );
    }
}

fn clip(area: Rect, to: Rect) -> Rect {
    let (x, y) = (area.x.max(to.x), area.y.max(to.y));
    let right = (area.x + area.width).min(to.x + to.width);
    let bottom = (area.y + area.height).min(to.y + to.height);
    Rect {
        x,
        y,
        width: (right - x).max(0),
        height: (bottom - y).max(0),
    }
}

pub fn draw_document(
    target: &mut ViewPixels<'_>,
    cache: &RenderCache,
    placement: Placement,
    theme: (&Palette, &Roles),
) {
    let (palette, roles) = theme;
    let background = pack(palette.resolve(roles.window).unwrap_or_default());
    let Some(image) = cache.image() else {
        target.rect(target.clip, background);
        return;
    };
    let shown = Rect {
        x: placement.origin.0,
        y: placement.origin.1,
        width: image.canvas.0 * placement.zoom,
        height: image.canvas.1 * placement.zoom,
    };
    for band in around(target.clip, shown) {
        target.rect(band, background);
    }
    let visible = clip(shown, target.clip);
    if visible.width <= 0 || visible.height <= 0 {
        return;
    }
    let source = |at: i32, origin: i32, shown: i32, size: i32| {
        let offset = i64::from(at - origin) * i64::from(size) / i64::from(shown.max(1));
        usize::try_from(offset).unwrap_or(0)
    };
    let columns: Vec<usize> = (visible.x..visible.x + visible.width)
        .map(|x| source(x, shown.x, shown.width, image.size.0))
        .collect();
    let stride = usize::try_from(image.size.0).unwrap_or(0);
    let left = usize::try_from(visible.x).unwrap_or(0);
    let mut drawn: Option<(usize, usize)> = None;
    for y in visible.y..visible.y + visible.height {
        let row = source(y, shown.y, shown.height, image.size.1);
        let start = usize::try_from(y).unwrap_or(0) * target.width + left;
        if start + columns.len() > target.out.len() {
            continue;
        }
        if let Some((_, from)) = drawn.filter(|(previous, _)| *previous == row) {
            target.out.copy_within(from..from + columns.len(), start);
            continue;
        }
        let out = &mut target.out[start..start + columns.len()];
        let cells = image
            .cells
            .get(row * stride..(row + 1) * stride)
            .unwrap_or(&[]);
        let mut last: Option<(Paint, u32)> = None;
        for (pixel, column) in out.iter_mut().zip(&columns) {
            let Some(paint) = cells.get(*column).copied() else {
                continue;
            };
            let color = match last {
                Some((seen, color)) if seen == paint => color,
                _ => pack(image.palette.resolve(paint).unwrap_or_default()),
            };
            last = Some((paint, color));
            *pixel = color;
        }
        drawn = Some((row, start));
    }
}

fn around(area: Rect, hole: Rect) -> [Rect; 4] {
    let hole = clip(hole, area);
    let bottom = hole.y + hole.height;
    [
        Rect {
            height: hole.y - area.y,
            ..area
        },
        Rect {
            y: bottom,
            height: area.y + area.height - bottom,
            ..area
        },
        Rect {
            y: hole.y,
            width: hole.x - area.x,
            height: hole.height,
            ..area
        },
        Rect {
            x: hole.x + hole.width,
            y: hole.y,
            width: area.x + area.width - hole.x - hole.width,
            height: hole.height,
        },
    ]
}

pub fn draw_grid(
    target: &mut ViewPixels<'_>,
    placement: Placement,
    canvas: (i32, i32),
    color: u32,
) {
    if placement.zoom < 6 {
        return;
    }
    let z = placement.zoom;
    for x in 0..=canvas.0 {
        let wx = placement.origin.0 + x * z;
        target.rect(
            Rect {
                x: wx,
                y: placement.origin.1,
                width: 1,
                height: canvas.1 * z,
            },
            color,
        );
    }
    for y in 0..=canvas.1 {
        let wy = placement.origin.1 + y * z;
        target.rect(
            Rect {
                x: placement.origin.0,
                y: wy,
                width: canvas.0 * z,
                height: 1,
            },
            color,
        );
    }
}

pub fn draw_selection(
    target: &mut ViewPixels<'_>,
    placement: Placement,
    selection: &Selection,
    colors: (u32, u32),
    time_ms: u64,
) {
    let Some((x0, y0, x1, y1)) = selection.bounds() else {
        return;
    };
    let z = placement.zoom;
    let phase = i32::try_from(time_ms / 120 % 8).unwrap_or(0);
    let dash = |target: &mut ViewPixels<'_>, wx: i32, wy: i32| {
        let on = (wx + wy + phase).rem_euclid(8) < 4;
        target.set(wx, wy, if on { colors.0 } else { colors.1 });
    };
    for y in y0..=y1 {
        for x in x0..=x1 {
            if !selection.contains(x, y) {
                continue;
            }
            let left = placement.origin.0 + x * z;
            let top = placement.origin.1 + y * z;
            for i in 0..z {
                if !selection.contains(x, y - 1) {
                    dash(target, left + i, top);
                }
                if !selection.contains(x, y + 1) {
                    dash(target, left + i, top + z - 1);
                }
                if !selection.contains(x - 1, y) {
                    dash(target, left, top + i);
                }
                if !selection.contains(x + 1, y) {
                    dash(target, left + z - 1, top + i);
                }
            }
        }
    }
}

pub fn draw_cursor(target: &mut ViewPixels<'_>, placement: Placement, at: (i32, i32), color: u32) {
    let z = placement.zoom;
    let left = placement.origin.0 + at.0 * z - 1;
    let top = placement.origin.1 + at.1 * z - 1;
    let side = z + 2;
    target.rect(
        Rect {
            x: left,
            y: top,
            width: side,
            height: 1,
        },
        color,
    );
    target.rect(
        Rect {
            x: left,
            y: top + side - 1,
            width: side,
            height: 1,
        },
        color,
    );
    target.rect(
        Rect {
            x: left,
            y: top,
            width: 1,
            height: side,
        },
        color,
    );
    target.rect(
        Rect {
            x: left + side - 1,
            y: top,
            width: 1,
            height: side,
        },
        color,
    );
}
