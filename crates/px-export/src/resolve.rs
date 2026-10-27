use px_core::{CanvasView, Paint, Palette};

use crate::error::ExportError;
use crate::scale::Scale;

pub fn indexed_len(
    view: &CanvasView<'_>,
    scale: Scale,
    channels: usize,
) -> Result<usize, ExportError> {
    let size = view.size();
    let width = usize::from(scale.apply(size.width()).ok_or(ExportError::TooLarge)?);
    let height = usize::from(scale.apply(size.height()).ok_or(ExportError::TooLarge)?);
    width
        .checked_mul(height)
        .and_then(|n| n.checked_mul(channels))
        .ok_or(ExportError::TooLarge)
}

pub fn resolve_rgb(
    view: &CanvasView<'_>,
    palette: &Palette,
    scale: Scale,
    out: &mut [u8],
) -> Result<(), ExportError> {
    upscale(view, scale, 3, out, |paint, pixel| {
        let rgb = palette.resolve(paint)?;
        pixel.copy_from_slice(&[rgb.r, rgb.g, rgb.b]);
        Ok(())
    })
}

pub(crate) fn upscale(
    view: &CanvasView<'_>,
    scale: Scale,
    channels: usize,
    out: &mut [u8],
    mut write: impl FnMut(Paint, &mut [u8]) -> Result<(), ExportError>,
) -> Result<(), ExportError> {
    if out.len() != indexed_len(view, scale, channels)? {
        return Err(ExportError::TooLarge);
    }
    let factor = usize::from(scale.get());
    let width = usize::from(view.size().width());
    let row_len = width * factor * channels;
    for (source_row, cells) in view.cells().chunks_exact(width).enumerate() {
        let band = &mut out[source_row * row_len * factor..(source_row + 1) * row_len * factor];
        let (first_row, rest) = band.split_at_mut(row_len);
        for (cell, block) in cells
            .iter()
            .zip(first_row.chunks_exact_mut(factor * channels))
        {
            let (pixel, repeats) = block.split_at_mut(channels);
            write(*cell, pixel)?;
            for copy in repeats.chunks_exact_mut(channels) {
                copy.copy_from_slice(pixel);
            }
        }
        for row in rest.chunks_exact_mut(row_len) {
            row.copy_from_slice(first_row);
        }
    }
    Ok(())
}
