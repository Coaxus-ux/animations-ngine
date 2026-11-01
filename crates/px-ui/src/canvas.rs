use px_core::{Paint, Rgb};
use px_raster::Plot;

use crate::area::{Area, Rect, rect};
use crate::bitmap::Bitmap;

#[derive(Clone, Debug, PartialEq)]
pub enum Overlay {
    Box(Rect, Rgb),
    Image(Rect, Vec<Rgb>),
}

#[derive(Clone, Debug)]
pub struct UiCanvas {
    width: i32,
    height: i32,
    cells: Vec<Paint>,
    clip: Rect,
    overlays: Vec<Overlay>,
    top: Vec<Option<Paint>>,
    top_overlays: Vec<Overlay>,
    on_top: bool,
}

impl UiCanvas {
    #[must_use]
    pub fn new(width: i32, height: i32, background: Paint) -> Self {
        let mut canvas = Self {
            width: 0,
            height: 0,
            cells: Vec::new(),
            clip: Rect::default(),
            overlays: Vec::new(),
            top: Vec::new(),
            top_overlays: Vec::new(),
            on_top: false,
        };
        canvas.reset(width, height, background);
        canvas
    }

    pub fn reset(&mut self, width: i32, height: i32, background: Paint) {
        let (width, height) = (width.max(1), height.max(1));
        let count = usize::try_from(width * height).unwrap_or(0);
        self.cells.clear();
        self.cells.resize(count, background);
        self.top.clear();
        self.top.resize(count, None);
        self.width = width;
        self.height = height;
        self.clip = self.bounds();
        self.overlays.clear();
        self.top_overlays.clear();
        self.on_top = false;
    }

    pub fn set_on_top(&mut self, on_top: bool) -> bool {
        core::mem::replace(&mut self.on_top, on_top)
    }

    #[must_use]
    pub fn top(&self) -> &[Option<Paint>] {
        &self.top
    }

    #[must_use]
    pub fn top_overlays(&self) -> &[Overlay] {
        &self.top_overlays
    }

    #[must_use]
    pub const fn width(&self) -> i32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> i32 {
        self.height
    }

    #[must_use]
    pub const fn bounds(&self) -> Rect {
        rect(0, 0, self.width, self.height)
    }

    #[must_use]
    pub fn cells(&self) -> &[Paint] {
        &self.cells
    }

    #[must_use]
    pub fn overlays(&self) -> &[Overlay] {
        &self.overlays
    }

    pub fn clip_to(&mut self, area: Rect) -> Rect {
        let clip = area.intersect(self.bounds());
        core::mem::replace(&mut self.clip, clip)
    }

    pub fn restore_clip(&mut self, clip: Rect) {
        self.clip = clip;
    }

    #[must_use]
    pub fn get(&self, x: i32, y: i32) -> Option<Paint> {
        self.index(x, y).map(|index| self.cells[index])
    }

    pub fn pixel(&mut self, x: i32, y: i32, paint: Paint) {
        if self.clip.contains(x, y)
            && let Some(index) = self.index(x, y)
        {
            if self.on_top {
                self.top[index] = Some(paint);
            } else {
                self.cells[index] = paint;
            }
        }
    }

    pub fn fill(&mut self, area: Rect, paint: Paint) {
        let area = area.intersect(self.clip);
        for y in area.y..area.bottom() {
            if let (Some(start), Some(end)) =
                (self.index(area.x, y), self.index(area.right() - 1, y))
            {
                if self.on_top {
                    self.top[start..=end].fill(Some(paint));
                } else {
                    self.cells[start..=end].fill(paint);
                }
            }
        }
    }

    pub fn dither(&mut self, area: Rect, paint: Paint) {
        let area = area.intersect(self.clip);
        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                if (x + y) % 2 == 0 {
                    self.pixel(x, y, paint);
                }
            }
        }
    }

    pub fn frame(&mut self, area: Rect, paint: Paint) {
        if area.is_empty() {
            return;
        }
        self.fill(rect(area.x, area.y, area.width, 1), paint);
        self.fill(rect(area.x, area.bottom() - 1, area.width, 1), paint);
        self.fill(rect(area.x, area.y, 1, area.height), paint);
        self.fill(rect(area.right() - 1, area.y, 1, area.height), paint);
    }

    pub fn bevel(&mut self, area: Rect, light: Paint, dark: Paint, sunken: bool) {
        let (top, bottom) = if sunken { (dark, light) } else { (light, dark) };
        self.fill(rect(area.x, area.y, area.width, 1), top);
        self.fill(rect(area.x, area.y, 1, area.height), top);
        self.fill(rect(area.x, area.bottom() - 1, area.width, 1), bottom);
        self.fill(rect(area.right() - 1, area.y, 1, area.height), bottom);
    }

    pub fn blit(&mut self, bitmap: &Bitmap, x: i32, y: i32, tint: Option<Paint>) {
        for (row, column, paint) in bitmap.opaque() {
            self.pixel(x + column, y + row, tint.unwrap_or(paint));
        }
    }

    pub fn rgb_box(&mut self, area: Rect, color: Rgb) {
        let area = area.intersect(self.clip);
        if !area.is_empty() {
            self.overlay(Overlay::Box(area, color));
        }
    }

    pub fn rgb_image(&mut self, area: Rect, pixels: &[Rgb]) {
        let visible = area.intersect(self.clip);
        let stride = usize::try_from(area.width).unwrap_or(0);
        if visible.is_empty() || pixels.len() < stride * usize::try_from(area.height).unwrap_or(0) {
            return;
        }
        let column = usize::try_from(visible.x - area.x).unwrap_or(0);
        let width = usize::try_from(visible.width).unwrap_or(0);
        let cropped = (visible.y..visible.bottom())
            .filter_map(|y| usize::try_from(y - area.y).ok())
            .flat_map(|row| pixels[row * stride + column..row * stride + column + width].iter())
            .copied()
            .collect();
        self.overlay(Overlay::Image(visible, cropped));
    }

    fn overlay(&mut self, overlay: Overlay) {
        if self.on_top {
            self.top_overlays.push(overlay);
        } else {
            self.overlays.push(overlay);
        }
    }

    pub fn pen(&mut self, paint: Paint) -> Pen<'_> {
        Pen {
            canvas: self,
            paint,
        }
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.width && y < self.height)
            .then(|| usize::try_from(y * self.width + x).unwrap_or(0))
    }
}

pub struct Pen<'c> {
    canvas: &'c mut UiCanvas,
    paint: Paint,
}

impl Plot for Pen<'_> {
    fn width(&self) -> i32 {
        self.canvas.width
    }

    fn height(&self) -> i32 {
        self.canvas.height
    }

    fn span(&mut self, y: i32, x_start: i32, x_end: i32) {
        self.canvas
            .fill(rect(x_start, y, x_end - x_start, 1), self.paint);
    }
}
