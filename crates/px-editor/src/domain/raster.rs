use px_core::Paint;
use px_engine::Cel;
use px_raster::Plot;

/// One cel being edited, as a canvas-sized image in scene coordinates: what tools paint
/// into. Pixels outside the canvas aren't editable here and are kept as they were.
#[derive(Clone, Debug, PartialEq)]
pub struct Raster {
    width: i32,
    height: i32,
    pixels: Vec<Option<Paint>>,
}

impl Raster {
    #[must_use]
    pub fn new(width: i32, height: i32) -> Self {
        let (width, height) = (width.max(1), height.max(1));
        Self {
            width,
            height,
            pixels: vec![None; usize::try_from(width * height).unwrap_or(0)],
        }
    }

    /// The pixels of `cel`, whose layer's origin is at `origin` on the canvas.
    #[must_use]
    pub fn from_cel(cel: Option<&Cel>, origin: (i32, i32), width: i32, height: i32) -> Self {
        let mut raster = Self::new(width, height);
        if let Some(cel) = cel {
            let columns = usize::from(cel.width.max(1));
            for (row, paints) in (0i32..).zip(cel.pixels.chunks(columns)) {
                for (column, paint) in (0i32..).zip(paints) {
                    let x = origin.0 + cel.x + column;
                    let y = origin.1 + cel.y + row;
                    raster.set(x, y, *paint);
                }
            }
        }
        raster
    }

    /// A cel starting at `frame` holding these pixels, relative to the layer's `origin`.
    #[must_use]
    pub fn to_cel(&self, frame: u32, origin: (i32, i32)) -> Cel {
        Cel {
            frame,
            x: -origin.0,
            y: -origin.1,
            width: u16::try_from(self.width).unwrap_or(u16::MAX),
            height: u16::try_from(self.height).unwrap_or(u16::MAX),
            pixels: self.pixels.clone(),
        }
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
    pub fn get(&self, x: i32, y: i32) -> Option<Paint> {
        self.index(x, y).and_then(|index| self.pixels[index])
    }

    pub fn set(&mut self, x: i32, y: i32, paint: Option<Paint>) {
        if let Some(index) = self.index(x, y) {
            self.pixels[index] = paint;
        }
    }

    #[must_use]
    pub fn contains(&self, x: i32, y: i32) -> bool {
        self.index(x, y).is_some()
    }

    /// Fills the 4-connected region of pixels equal to the one at `(x, y)` (or, `global`,
    /// every such pixel), only where `allowed` says so.
    pub fn flood(
        &mut self,
        x: i32,
        y: i32,
        paint: Option<Paint>,
        global: bool,
        allowed: impl Fn(i32, i32) -> bool,
    ) {
        let Some(start) = self.index(x, y) else {
            return;
        };
        let target = self.pixels[start];
        if target == paint {
            return;
        }
        if global {
            for py in 0..self.height {
                for px in 0..self.width {
                    if self.get(px, py) == target && allowed(px, py) {
                        self.set(px, py, paint);
                    }
                }
            }
            return;
        }
        let mut stack = vec![(x, y)];
        while let Some((px, py)) = stack.pop() {
            if !self.contains(px, py) || self.get(px, py) != target || !allowed(px, py) {
                continue;
            }
            self.set(px, py, paint);
            stack.extend([(px + 1, py), (px - 1, py), (px, py + 1), (px, py - 1)]);
        }
    }

    /// A `Plot` that sets pixels to `paint` where `allowed` says so.
    pub fn brush<F: Fn(i32, i32) -> bool>(
        &mut self,
        paint: Option<Paint>,
        allowed: F,
    ) -> Brush<'_, F> {
        Brush {
            raster: self,
            paint,
            allowed,
        }
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.width && y < self.height)
            .then(|| usize::try_from(y * self.width + x).unwrap_or(0))
    }
}

pub struct Brush<'r, F> {
    raster: &'r mut Raster,
    paint: Option<Paint>,
    allowed: F,
}

impl<F: Fn(i32, i32) -> bool> Plot for Brush<'_, F> {
    fn width(&self) -> i32 {
        self.raster.width
    }

    fn height(&self) -> i32 {
        self.raster.height
    }

    fn span(&mut self, y: i32, x_start: i32, x_end: i32) {
        for x in x_start.max(0)..x_end.min(self.raster.width) {
            if (self.allowed)(x, y) {
                self.raster.set(x, y, self.paint);
            }
        }
    }
}
