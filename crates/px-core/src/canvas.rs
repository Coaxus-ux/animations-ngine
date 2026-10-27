use core::fmt;

use crate::palette::Paint;

/// Largest supported canvas width or height, in pixels.
pub const MAX_CANVAS_SIDE: u16 = 2048;

/// A validated canvas size (1..=[`MAX_CANVAS_SIDE`] on each side).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Size {
    width: u16,
    height: u16,
}

/// Why a canvas could not be created.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanvasError {
    /// Width or height is 0 or above [`MAX_CANVAS_SIDE`].
    InvalidSize {
        /// Requested width.
        width: u16,
        /// Requested height.
        height: u16,
    },
    /// The cell buffer does not hold exactly `width × height` cells.
    BufferLength {
        /// Cells required.
        expected: usize,
        /// Cells provided.
        actual: usize,
    },
}

impl Size {
    /// Validates a size.
    ///
    /// # Errors
    ///
    /// [`CanvasError::InvalidSize`] when a side is 0 or too large.
    pub const fn new(width: u16, height: u16) -> Result<Self, CanvasError> {
        if width == 0 || height == 0 || width > MAX_CANVAS_SIDE || height > MAX_CANVAS_SIDE {
            return Err(CanvasError::InvalidSize { width, height });
        }
        Ok(Self { width, height })
    }

    /// Width in pixels.
    #[must_use]
    pub const fn width(self) -> u16 {
        self.width
    }

    /// Height in pixels.
    #[must_use]
    pub const fn height(self) -> u16 {
        self.height
    }

    /// Number of cells (`width × height`); cannot overflow given the side limit.
    #[must_use]
    pub const fn cell_count(self) -> usize {
        self.width as usize * self.height as usize
    }

    /// The cell index of an in-bounds pixel, or `None` when outside the canvas.
    fn index(self, x: i32, y: i32) -> Option<usize> {
        let x = usize::try_from(x)
            .ok()
            .filter(|x| *x < usize::from(self.width))?;
        let y = usize::try_from(y)
            .ok()
            .filter(|y| *y < usize::from(self.height))?;
        Some(y * usize::from(self.width) + x)
    }

    fn check_buffer(self, actual: usize) -> Result<(), CanvasError> {
        let expected = self.cell_count();
        if actual == expected {
            Ok(())
        } else {
            Err(CanvasError::BufferLength { expected, actual })
        }
    }
}

/// A writable canvas over a caller-owned cell buffer (row-major, top-left origin).
///
/// Drawing outside the canvas is clipped silently: off-canvas geometry is normal.
#[derive(Debug)]
pub struct CanvasMut<'a> {
    size: Size,
    cells: &'a mut [Paint],
}

impl<'a> CanvasMut<'a> {
    /// Wraps a buffer of exactly `size.cell_count()` cells.
    ///
    /// # Errors
    ///
    /// [`CanvasError::BufferLength`] when the buffer has the wrong length.
    pub fn new(size: Size, cells: &'a mut [Paint]) -> Result<Self, CanvasError> {
        size.check_buffer(cells.len())?;
        Ok(Self { size, cells })
    }

    /// The canvas size.
    #[must_use]
    pub const fn size(&self) -> Size {
        self.size
    }

    /// A read-only view of the same cells.
    #[must_use]
    pub fn view(&self) -> CanvasView<'_> {
        CanvasView {
            size: self.size,
            cells: self.cells,
        }
    }

    /// Paints every cell.
    pub fn fill(&mut self, paint: Paint) {
        self.cells.fill(paint);
    }

    /// Paints one pixel; ignored when outside the canvas.
    pub fn set(&mut self, x: i32, y: i32, paint: Paint) {
        if let Some(index) = self.size.index(x, y) {
            self.cells[index] = paint;
        }
    }

    /// Paints row `y` from `x_start` (inclusive) to `x_end` (exclusive), clipped.
    pub fn fill_span(&mut self, y: i32, x_start: i32, x_end: i32, paint: Paint) {
        let width = i32::from(self.size.width);
        let (start, end) = (x_start.max(0), x_end.min(width));
        if start >= end {
            return;
        }
        let (Some(first), Some(last)) = (self.size.index(start, y), self.size.index(end - 1, y))
        else {
            return;
        };
        debug_assert!(first <= last && last < self.cells.len());
        self.cells[first..=last].fill(paint);
    }
}

/// A read-only canvas over a cell buffer.
#[derive(Clone, Copy, Debug)]
pub struct CanvasView<'a> {
    size: Size,
    cells: &'a [Paint],
}

impl<'a> CanvasView<'a> {
    /// Wraps a buffer of exactly `size.cell_count()` cells.
    ///
    /// # Errors
    ///
    /// [`CanvasError::BufferLength`] when the buffer has the wrong length.
    pub fn new(size: Size, cells: &'a [Paint]) -> Result<Self, CanvasError> {
        size.check_buffer(cells.len())?;
        Ok(Self { size, cells })
    }

    /// The canvas size.
    #[must_use]
    pub const fn size(&self) -> Size {
        self.size
    }

    /// All cells, row-major.
    #[must_use]
    pub const fn cells(&self) -> &'a [Paint] {
        self.cells
    }

    /// The paint at a pixel, or `None` outside the canvas.
    #[must_use]
    pub fn get(&self, x: i32, y: i32) -> Option<Paint> {
        self.size.index(x, y).map(|index| self.cells[index])
    }
}

impl fmt::Display for CanvasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSize { width, height } => write!(
                f,
                "canvas size {width}×{height} is invalid (each side must be 1..={MAX_CANVAS_SIDE})"
            ),
            Self::BufferLength { expected, actual } => {
                write!(f, "canvas buffer has {actual} cells, expected {expected}")
            }
        }
    }
}
