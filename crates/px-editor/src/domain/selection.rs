use px_raster::{Plot, Rect, fill_polygon, fill_rect};

use super::raster::Raster;

/// Which canvas pixels tools may change; empty means all of them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Selection {
    width: i32,
    height: i32,
    mask: Vec<bool>,
    any: bool,
}

impl Selection {
    #[must_use]
    pub fn none(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            mask: vec![false; usize::try_from(width.max(0) * height.max(0)).unwrap_or(0)],
            any: false,
        }
    }

    /// The pixels of the box spanned by two corners (inclusive).
    #[must_use]
    pub fn rectangle(width: i32, height: i32, a: (i32, i32), b: (i32, i32)) -> Self {
        let mut selection = Self::none(width, height);
        let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
        let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
        fill_rect(
            &mut selection,
            Rect {
                x: x0,
                y: y0,
                width: x1 - x0 + 1,
                height: y1 - y0 + 1,
            },
        );
        selection.refresh();
        selection
    }

    /// The pixels inside the polygon traced by a lasso.
    #[must_use]
    pub fn lasso(width: i32, height: i32, points: &[(i32, i32)]) -> Self {
        let mut selection = Self::none(width, height);
        let corners: Vec<[f64; 2]> = points
            .iter()
            .map(|(x, y)| [f64::from(*x) + 0.5, f64::from(*y) + 0.5])
            .collect();
        fill_polygon(&mut selection, &corners);
        for (x, y) in points {
            selection.mark(*x, *y);
        }
        selection.refresh();
        selection
    }

    #[must_use]
    pub fn of_pixels(
        width: i32,
        height: i32,
        points: impl IntoIterator<Item = (i32, i32)>,
    ) -> Self {
        let mut selection = Self::none(width, height);
        for (x, y) in points {
            selection.mark(x, y);
        }
        selection.refresh();
        selection
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        !self.any
    }

    /// Whether tools may change `(x, y)`: inside the selection, or anywhere without one.
    #[must_use]
    pub fn allows(&self, x: i32, y: i32) -> bool {
        !self.any || self.contains(x, y)
    }

    #[must_use]
    pub fn contains(&self, x: i32, y: i32) -> bool {
        self.index(x, y).is_some_and(|index| self.mask[index])
    }

    #[must_use]
    pub fn bounds(&self) -> Option<(i32, i32, i32, i32)> {
        let mut bounds: Option<(i32, i32, i32, i32)> = None;
        for y in 0..self.height {
            for x in 0..self.width {
                if self.contains(x, y) {
                    let (x0, y0, x1, y1) = bounds.unwrap_or((x, y, x, y));
                    bounds = Some((x0.min(x), y0.min(y), x1.max(x), y1.max(y)));
                }
            }
        }
        bounds
    }

    /// Whether `(x, y)` is selected but a neighbor isn't: where marching ants go.
    #[must_use]
    pub fn is_edge(&self, x: i32, y: i32) -> bool {
        self.contains(x, y)
            && [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .any(|(dx, dy)| !self.contains(x + dx, y + dy))
    }

    #[must_use]
    pub fn moved(&self, dx: i32, dy: i32) -> Self {
        let mut moved = Self::none(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                if self.contains(x, y) {
                    moved.mark(x + dx, y + dy);
                }
            }
        }
        moved.refresh();
        moved
    }

    /// The selected pixels of `raster`, lifted out (left empty in it).
    pub fn lift(&self, raster: &mut Raster) -> Raster {
        let mut lifted = Raster::new(raster.width(), raster.height());
        for y in 0..self.height {
            for x in 0..self.width {
                if self.contains(x, y) {
                    lifted.set(x, y, raster.get(x, y));
                    raster.set(x, y, None);
                }
            }
        }
        lifted
    }

    /// Mirrors the selected pixels of `raster` inside the selection's box.
    pub fn flip(&self, raster: &mut Raster, horizontal: bool) {
        let Some((x0, y0, x1, y1)) = self.bounds() else {
            return;
        };
        let lifted = self.lift(raster);
        for y in y0..=y1 {
            for x in x0..=x1 {
                if self.contains(x, y) {
                    let (sx, sy) = if horizontal {
                        (x0 + x1 - x, y)
                    } else {
                        (x, y0 + y1 - y)
                    };
                    if let Some(paint) = lifted.get(sx, sy) {
                        raster.set(x, y, Some(paint));
                    }
                }
            }
        }
    }

    fn mark(&mut self, x: i32, y: i32) {
        if let Some(index) = self.index(x, y) {
            self.mask[index] = true;
        }
    }

    fn refresh(&mut self) {
        self.any = self.mask.iter().any(|selected| *selected);
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.width && y < self.height)
            .then(|| usize::try_from(y * self.width + x).unwrap_or(0))
    }
}

impl Plot for Selection {
    fn width(&self) -> i32 {
        self.width
    }

    fn height(&self) -> i32 {
        self.height
    }

    fn span(&mut self, y: i32, x_start: i32, x_end: i32) {
        for x in x_start..x_end {
            self.mark(x, y);
        }
    }
}
