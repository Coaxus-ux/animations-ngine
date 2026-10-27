use core::fmt;

use crate::palette::Paint;

pub const MAX_CANVAS_SIDE: u16 = 2048;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Size {
    width: u16,
    height: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanvasError {
    InvalidSize { width: u16, height: u16 },
    BufferLength { expected: usize, actual: usize },
}

impl Size {
    pub const fn new(width: u16, height: u16) -> Result<Self, CanvasError> {
        if width == 0 || height == 0 || width > MAX_CANVAS_SIDE || height > MAX_CANVAS_SIDE {
            return Err(CanvasError::InvalidSize { width, height });
        }
        Ok(Self { width, height })
    }

    #[must_use]
    pub const fn width(self) -> u16 {
        self.width
    }

    #[must_use]
    pub const fn height(self) -> u16 {
        self.height
    }

    #[must_use]
    pub const fn cell_count(self) -> usize {
        self.width as usize * self.height as usize
    }

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

#[derive(Debug)]
pub struct CanvasMut<'a> {
    size: Size,
    cells: &'a mut [Paint],
}

impl<'a> CanvasMut<'a> {
    pub fn new(size: Size, cells: &'a mut [Paint]) -> Result<Self, CanvasError> {
        size.check_buffer(cells.len())?;
        Ok(Self { size, cells })
    }

    #[must_use]
    pub const fn size(&self) -> Size {
        self.size
    }

    #[must_use]
    pub fn view(&self) -> CanvasView<'_> {
        CanvasView {
            size: self.size,
            cells: self.cells,
        }
    }

    pub fn fill(&mut self, paint: Paint) {
        self.cells.fill(paint);
    }

    pub fn set(&mut self, x: i32, y: i32, paint: Paint) {
        if let Some(index) = self.size.index(x, y) {
            self.cells[index] = paint;
        }
    }

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

#[derive(Clone, Copy, Debug)]
pub struct CanvasView<'a> {
    size: Size,
    cells: &'a [Paint],
}

impl<'a> CanvasView<'a> {
    pub fn new(size: Size, cells: &'a [Paint]) -> Result<Self, CanvasError> {
        size.check_buffer(cells.len())?;
        Ok(Self { size, cells })
    }

    #[must_use]
    pub const fn size(&self) -> Size {
        self.size
    }

    #[must_use]
    pub const fn cells(&self) -> &'a [Paint] {
        self.cells
    }

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
