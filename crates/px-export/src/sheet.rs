use crate::error::ExportError;
use crate::rgba::RgbaImage;

/// How frames are arranged on a sprite sheet (Aseprite's sheet types).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SheetLayout {
    HorizontalStrip,
    VerticalStrip,
    Rows { columns: u16 },
    Columns { rows: u16 },
}

/// Empty pixels around the sheet (`border`) and between frames (`shape`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Padding {
    pub border: u16,
    pub shape: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Cells per row and per column, and whether cells fill a column before moving on.
struct Grid {
    columns: u32,
    rows: u32,
    column_major: bool,
}

impl Grid {
    fn for_layout(count: u32, layout: SheetLayout) -> Result<Self, ExportError> {
        let ceil_div = |count: u32, by: u32| count.div_ceil(by);
        let (columns, rows, column_major) = match layout {
            SheetLayout::HorizontalStrip => (count, count.min(1), false),
            SheetLayout::VerticalStrip => (count.min(1), count, true),
            SheetLayout::Rows { columns: 0 } => {
                return Err(ExportError::Sheet(
                    "rows layout needs at least one column".into(),
                ));
            }
            SheetLayout::Columns { rows: 0 } => {
                return Err(ExportError::Sheet(
                    "columns layout needs at least one row".into(),
                ));
            }
            SheetLayout::Rows { columns } => {
                let columns = u32::from(columns).min(count);
                (columns, ceil_div(count, columns.max(1)), false)
            }
            SheetLayout::Columns { rows } => {
                let rows = u32::from(rows).min(count);
                (ceil_div(count, rows.max(1)), rows, true)
            }
        };
        Ok(Self {
            columns,
            rows,
            column_major,
        })
    }

    fn position(&self, index: u32) -> (u32, u32) {
        if self.column_major {
            (index / self.rows, index % self.rows)
        } else {
            (index % self.columns, index / self.columns)
        }
    }
}

/// The sheet size and one cell per frame for `count` frames of `frame` pixels.
///
/// # Errors
/// `ExportError::TooLarge` when the sheet overflows `u32`; `ExportError::Sheet` for a zero
/// column or row count.
pub fn place(
    count: usize,
    frame: (u16, u16),
    layout: SheetLayout,
    padding: Padding,
) -> Result<((u32, u32), Vec<CellRect>), ExportError> {
    let count = u32::try_from(count).map_err(|_| ExportError::TooLarge)?;
    let grid = Grid::for_layout(count, layout)?;
    let (w, h) = (u32::from(frame.0), u32::from(frame.1));
    let (border, shape) = (u32::from(padding.border), u32::from(padding.shape));
    let pitch = |side: u32| side.checked_add(shape).ok_or(ExportError::TooLarge);
    let (pitch_x, pitch_y) = (pitch(w)?, pitch(h)?);
    let extent = |cells: u32, pitch: u32| -> Result<u32, ExportError> {
        if cells == 0 {
            return Ok(0);
        }
        cells
            .checked_mul(pitch)
            .and_then(|span| span.checked_sub(shape))
            .and_then(|span| span.checked_add(border * 2))
            .ok_or(ExportError::TooLarge)
    };
    let size = (extent(grid.columns, pitch_x)?, extent(grid.rows, pitch_y)?);
    let cells = (0..count)
        .map(|index| {
            let (column, row) = grid.position(index);
            Ok(CellRect {
                x: border + column * pitch_x,
                y: border + row * pitch_y,
                w,
                h,
            })
        })
        .collect::<Result<Vec<_>, ExportError>>()?;
    Ok((size, cells))
}

/// Lays `frames` (all the same size) out on one image.
///
/// # Errors
/// As `place`, plus `ExportError::Sheet` when the frames differ in size.
pub fn compose(
    frames: &[RgbaImage],
    layout: SheetLayout,
    padding: Padding,
) -> Result<(RgbaImage, Vec<CellRect>), ExportError> {
    let Some(first) = frames.first() else {
        return Ok((RgbaImage::new(0, 0)?, Vec::new()));
    };
    let side = |value: u32| u16::try_from(value).map_err(|_| ExportError::TooLarge);
    let frame = (side(first.width)?, side(first.height)?);
    for (index, image) in frames.iter().enumerate().skip(1) {
        if (image.width, image.height) != (first.width, first.height) {
            return Err(ExportError::Sheet(format!(
                "frame {index} is {}x{} but frame 0 is {}x{}",
                image.width, image.height, first.width, first.height
            )));
        }
    }
    let ((width, height), cells) = place(frames.len(), frame, layout, padding)?;
    let mut sheet = RgbaImage::new(width, height)?;
    for (image, cell) in frames.iter().zip(&cells) {
        sheet.blit(image, cell.x, cell.y);
    }
    Ok((sheet, cells))
}
