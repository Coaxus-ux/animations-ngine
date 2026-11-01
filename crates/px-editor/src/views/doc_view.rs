use std::collections::BTreeSet;

use px_core::{Frame, Paint, Palette};
use px_engine::{FrameBuffer, Scene};
use px_ui::{Rect, Roles, fill, pack};

use crate::app::{DocumentId, ViewState};
use crate::domain::visibility::with_hidden_layers;
use crate::domain::{Document, Selection};

/// The document rendered for one window: rebuilt only when the document, the frame or the
/// window's hidden layers change.
pub struct RenderCache {
    key: (Option<DocumentId>, u64, u32, BTreeSet<String>),
    scene: Option<Scene>,
    buffer: Option<FrameBuffer>,
}

impl RenderCache {
    #[must_use]
    pub fn new() -> Self {
        Self {
            key: (None, u64::MAX, 0, BTreeSet::new()),
            scene: None,
            buffer: None,
        }
    }

    /// Renders `view.frame` of the document unless it is already rendered.
    pub fn update(&mut self, id: DocumentId, document: &Document, view: &ViewState) {
        let key = (
            Some(id),
            document.revision(),
            view.frame,
            view.hidden.clone(),
        );
        if key == self.key {
            return;
        }
        self.key = key;
        let Some(scene) = document.scene() else {
            return;
        };
        let scene =
            with_hidden_layers(document, scene, &view.hidden).unwrap_or_else(|| scene.clone());
        let duration = scene.timing().duration.get().max(1);
        let mut buffer = FrameBuffer::new(&scene);
        if buffer
            .render(&scene, Frame::new(view.frame % duration))
            .is_err()
        {
            return;
        }
        self.scene = Some(scene);
        self.buffer = Some(buffer);
    }

    /// The rendered canvas: (width, height, paints, palette).
    #[must_use]
    pub fn image(&self) -> Option<(i32, i32, &[Paint], &Palette)> {
        let scene = self.scene.as_ref()?;
        let buffer = self.buffer.as_ref()?;
        let size = scene.size();
        Some((
            i32::from(size.width()),
            i32::from(size.height()),
            buffer.cells(),
            scene.palette(),
        ))
    }

    /// The shape that drew canvas pixel `(x, y)`.
    #[must_use]
    pub fn shape_at(&self, x: i32, y: i32) -> Option<usize> {
        self.buffer.as_ref()?.shape_at(x, y)
    }

    /// The paint shown at canvas pixel `(x, y)`.
    #[must_use]
    pub fn paint_at(&self, x: i32, y: i32) -> Option<Paint> {
        let (width, height, cells, _) = self.image()?;
        (x >= 0 && y >= 0 && x < width && y < height)
            .then(|| cells[usize::try_from(y * width + x).unwrap_or(0)])
    }
}

impl Default for RenderCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Where the canvas lands in a view: `zoom` window pixels per canvas pixel, its top-left
/// at `origin` (window pixels).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Placement {
    pub zoom: i32,
    pub origin: (i32, i32),
}

impl Placement {
    /// The view's zoom and pan, or the largest whole zoom that fits, centered.
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

    /// The canvas pixel under window pixel `(x, y)`.
    #[must_use]
    pub const fn to_canvas(self, x: i32, y: i32) -> (i32, i32) {
        (
            (x - self.origin.0).div_euclid(self.zoom),
            (y - self.origin.1).div_euclid(self.zoom),
        )
    }
}

/// The parts of one canvas view, in window pixels.
pub struct ViewPixels<'a> {
    pub out: &'a mut [u32],
    pub width: usize,
    pub clip: Rect,
}

impl ViewPixels<'_> {
    /// Fills canvas pixel `(x, y)`'s block with `color`, or every other window pixel of it
    /// (`dithered`), clipped to the view.
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

/// Draws the rendered canvas and the window's overlays (onion skin, grid, selection,
/// guide, cursor) into `target` at `placement`.
pub fn draw_document(
    target: &mut ViewPixels<'_>,
    cache: &RenderCache,
    placement: Placement,
    theme: (&Palette, &Roles),
) {
    let (palette, roles) = theme;
    target.rect(
        target.clip,
        pack(palette.resolve(roles.window).unwrap_or_default()),
    );
    let Some((width, height, cells, doc_palette)) = cache.image() else {
        return;
    };
    for y in 0..height {
        for x in 0..width {
            let paint = cells[usize::try_from(y * width + x).unwrap_or(0)];
            let color = pack(doc_palette.resolve(paint).unwrap_or_default());
            target.block(placement, (x, y), color, false);
        }
    }
}

/// Grid lines between canvas pixels, once pixels are big enough to see them.
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

/// Marching ants along the selection's edges: dashes that shift with `time_ms`.
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

/// A one-pixel outline around canvas pixel `at`.
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
