use px_core::Paint;
use px_raster::{Plot, Rect};

pub(crate) const FULL: u8 = 255;

pub(crate) struct Mask<'a> {
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) coverage: &'a mut [u8],
    pub(crate) temp: &'a mut [u8],
    pub(crate) distance: &'a mut [u16],
    pub(crate) noise: &'a mut [i8],
    pub(crate) paints: &'a mut [Paint],
    pub(crate) bbox: Option<[i32; 4]>,
}

impl Mask<'_> {
    pub(crate) fn index(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.width && y < self.height)
            .then(|| usize::try_from(y * self.width + x).unwrap_or(0))
    }

    pub(crate) fn alpha(&self, x: i32, y: i32) -> u8 {
        self.index(x, y).map_or(0, |index| self.coverage[index])
    }

    pub(crate) fn covered(&self, x: i32, y: i32) -> bool {
        self.alpha(x, y) > 0
    }

    pub(crate) fn set_alpha(&mut self, x: i32, y: i32, alpha: u8) {
        if let Some(index) = self.index(x, y) {
            self.coverage[index] = self.coverage[index].max(alpha);
            self.grow(y, x, x + 1);
        }
    }

    pub(crate) fn fill_paint(&mut self, rect: Rect, paint: Paint) {
        let (x0, x1) = (rect.x.max(0), (rect.x + rect.width).min(self.width));
        let (y0, y1) = (rect.y.max(0), (rect.y + rect.height).min(self.height));
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        for y in y0..y1 {
            if let (Some(first), Some(last)) = (self.index(x0, y), self.index(x1 - 1, y)) {
                self.coverage[first..=last].fill(FULL);
                self.paints[first..=last].fill(paint);
            }
        }
        self.grow(y0, x0, x1);
        self.grow(y1 - 1, x0, x1);
    }

    fn grow(&mut self, y: i32, x_start: i32, x_end: i32) {
        let [x0, y0, x1, y1] = self.bbox.unwrap_or([x_start, y, x_end, y + 1]);
        self.bbox = Some([x0.min(x_start), y0.min(y), x1.max(x_end), y1.max(y + 1)]);
    }

    pub(crate) fn clear(&mut self) {
        if let Some([x0, y0, x1, y1]) = self.bbox.take() {
            for y in y0..y1 {
                if let (Some(start), Some(_)) = (self.index(x0, y), self.index(x1 - 1, y)) {
                    let end = start + usize::try_from(x1 - x0).unwrap_or(0);
                    self.coverage[start..end].fill(0);
                }
            }
        }
    }

    pub(crate) fn pixels(&self) -> impl Iterator<Item = (i32, i32)> + use<> {
        let [x0, y0, x1, y1] = self.bbox.unwrap_or([0, 0, 0, 0]);
        (y0..y1).flat_map(move |y| (x0..x1).map(move |x| (x, y)))
    }

    pub(crate) fn clean(&mut self) {
        for (x, y) in self.pixels() {
            if let Some(index) = self.index(x, y) {
                let neighbors = self.neighbors(x, y);
                let alpha = self.coverage[index];
                self.temp[index] = match alpha {
                    0 if neighbors >= 6 => FULL,
                    a if a > 0 && neighbors < 2 => 0,
                    a => a,
                };
            }
        }
        for (x, y) in self.pixels() {
            if let Some(index) = self.index(x, y) {
                self.coverage[index] = self.temp[index];
            }
        }
    }

    pub(crate) fn neighbors(&self, x: i32, y: i32) -> usize {
        NEIGHBORS
            .iter()
            .filter(|(dx, dy)| self.covered(x + dx, y + dy))
            .count()
    }

    pub(crate) fn is_edge(&self, x: i32, y: i32) -> bool {
        self.covered(x, y)
            && [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .any(|(dx, dy)| !self.covered(x + dx, y + dy))
    }

    pub(crate) fn distance_field(&mut self) -> u16 {
        let Some([x0, y0, x1, y1]) = self.bbox else {
            return 1;
        };
        for (x, y) in self.pixels() {
            if let Some(index) = self.index(x, y) {
                self.distance[index] = if self.coverage[index] > 0 { FAR } else { 0 };
            }
        }
        for y in y0..y1 {
            for x in x0..x1 {
                self.relax(x, y, &FORWARD);
            }
        }
        let mut maximum = 1;
        for y in (y0..y1).rev() {
            for x in (x0..x1).rev() {
                maximum = maximum.max(self.relax(x, y, &BACKWARD));
            }
        }
        maximum
    }

    fn relax(&mut self, x: i32, y: i32, offsets: &[(i32, i32, u16); 4]) -> u16 {
        let Some(index) = self.index(x, y) else {
            return 0;
        };
        if self.distance[index] == 0 {
            return 0;
        }
        let [x0, y0, x1, y1] = self.bbox.unwrap_or_default();
        let mut best = FAR;
        for &(dx, dy, cost) in offsets {
            let (nx, ny) = (x + dx, y + dy);
            let neighbor = if nx >= x0 && nx < x1 && ny >= y0 && ny < y1 {
                self.index(nx, ny).map_or(0, |i| self.distance[i])
            } else {
                0
            };
            best = best.min(neighbor.saturating_add(cost));
        }
        self.distance[index] = self.distance[index].min(best).max(3);
        self.distance[index]
    }

    pub(crate) fn distance_at(&self, x: i32, y: i32) -> u16 {
        self.index(x, y)
            .filter(|_| self.covered(x, y))
            .map_or(0, |index| self.distance[index])
    }
}

const FAR: u16 = u16::MAX / 2;
const NEIGHBORS: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];
const FORWARD: [(i32, i32, u16); 4] = [(-1, 0, 3), (0, -1, 3), (-1, -1, 4), (1, -1, 4)];
const BACKWARD: [(i32, i32, u16); 4] = [(1, 0, 3), (0, 1, 3), (1, 1, 4), (-1, 1, 4)];

impl Plot for Mask<'_> {
    fn width(&self) -> i32 {
        self.width
    }

    fn height(&self) -> i32 {
        self.height
    }

    fn span(&mut self, y: i32, x_start: i32, x_end: i32) {
        let (start, end) = (x_start.max(0), x_end.min(self.width));
        if y < 0 || y >= self.height || start >= end {
            return;
        }
        if let (Some(first), Some(last)) = (self.index(start, y), self.index(end - 1, y)) {
            self.coverage[first..=last].fill(FULL);
            self.grow(y, start, end);
        }
    }
}
