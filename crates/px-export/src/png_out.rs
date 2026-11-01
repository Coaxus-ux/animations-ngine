use std::io::Write;

use px_core::{CanvasView, Palette, Rgb, Size};

use crate::error::ExportError;
use crate::resolve::{resolve_rgb, scaled_len, upscale};
use crate::scale::Scale;

pub fn write_png(
    writer: impl Write,
    view: &CanvasView<'_>,
    palette: &Palette,
    scale: Scale,
) -> Result<(), ExportError> {
    let mut rgb = vec![0; scaled_len(view.size(), scale, 3)?];
    resolve_rgb(view, palette, scale, &mut rgb)?;
    encode(writer, view.size(), scale, &rgb)
}

pub fn write_png_colors(
    writer: impl Write,
    size: Size,
    colors: &[Rgb],
    scale: Scale,
) -> Result<(), ExportError> {
    let mut rgb = vec![0; scaled_len(size, scale, 3)?];
    upscale(size, colors, scale, 3, &mut rgb, |color, pixel| {
        pixel.copy_from_slice(&[color.r, color.g, color.b]);
        Ok(())
    })?;
    encode(writer, size, scale, &rgb)
}

fn encode(writer: impl Write, size: Size, scale: Scale, rgb: &[u8]) -> Result<(), ExportError> {
    let width = scale.apply(size.width()).ok_or(ExportError::TooLarge)?;
    let height = scale.apply(size.height()).ok_or(ExportError::TooLarge)?;
    let mut encoder = png::Encoder::new(writer, u32::from(width), u32::from(height));
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let encoding = |error: png::EncodingError| ExportError::Encoding(error.to_string());
    let mut image = encoder.write_header().map_err(encoding)?;
    image.write_image_data(rgb).map_err(encoding)?;
    image.finish().map_err(encoding)
}
