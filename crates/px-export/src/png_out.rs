use std::io::Write;

use px_core::{CanvasView, Palette};

use crate::error::ExportError;
use crate::resolve::{indexed_len, resolve_rgb};
use crate::scale::Scale;

pub fn write_png(
    writer: impl Write,
    view: &CanvasView<'_>,
    palette: &Palette,
    scale: Scale,
) -> Result<(), ExportError> {
    let mut rgb = vec![0; indexed_len(view, scale, 3)?];
    resolve_rgb(view, palette, scale, &mut rgb)?;
    let size = view.size();
    let width = scale.apply(size.width()).ok_or(ExportError::TooLarge)?;
    let height = scale.apply(size.height()).ok_or(ExportError::TooLarge)?;
    let mut encoder = png::Encoder::new(writer, u32::from(width), u32::from(height));
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let encoding = |error: png::EncodingError| ExportError::Encoding(error.to_string());
    let mut image = encoder.write_header().map_err(encoding)?;
    image.write_image_data(&rgb).map_err(encoding)?;
    image.finish().map_err(encoding)
}
