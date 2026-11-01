use std::io::Write;

use px_core::Rgb;

use crate::error::ExportError;
use crate::scale::Scale;

const CHANNELS: usize = 4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<Option<Rgb>>,
}

impl RgbaImage {
    pub fn new(width: u32, height: u32) -> Result<Self, ExportError> {
        let count = usize::try_from(u64::from(width) * u64::from(height))
            .map_err(|_| ExportError::TooLarge)?;
        Ok(Self {
            width,
            height,
            pixels: vec![None; count],
        })
    }

    pub fn blit(&mut self, src: &Self, x: u32, y: u32) {
        let Some((columns, rows)) = self.overlap(src, x, y) else {
            return;
        };
        let (Some(dst_width), Some(src_width), Some(x), Some(y)) =
            (index(self.width), index(src.width), index(x), index(y))
        else {
            return;
        };
        for (row, src_row) in src.pixels.chunks_exact(src_width).take(rows).enumerate() {
            let start = (y + row) * dst_width + x;
            self.pixels[start..start + columns].copy_from_slice(&src_row[..columns]);
        }
    }

    fn overlap(&self, src: &Self, x: u32, y: u32) -> Option<(usize, usize)> {
        if src.width == 0 || x >= self.width || y >= self.height {
            return None;
        }
        let columns = index((self.width - x).min(src.width))?;
        let rows = index((self.height - y).min(src.height))?;
        Some((columns, rows))
    }
}

fn index(value: u32) -> Option<usize> {
    usize::try_from(value).ok()
}

pub fn write_png_rgba(
    writer: impl Write,
    image: &RgbaImage,
    scale: Scale,
) -> Result<(), ExportError> {
    let factor = u32::from(scale.get());
    let width = image
        .width
        .checked_mul(factor)
        .ok_or(ExportError::TooLarge)?;
    let height = image
        .height
        .checked_mul(factor)
        .ok_or(ExportError::TooLarge)?;
    let row_len = usize::try_from(width)
        .ok()
        .and_then(|width| width.checked_mul(CHANNELS))
        .ok_or(ExportError::TooLarge)?;
    let encoding = |error: png::EncodingError| ExportError::Encoding(error.to_string());
    let mut encoder = png::Encoder::new(writer, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut png = encoder.write_header().map_err(encoding)?;
    let mut stream = png.stream_writer().map_err(encoding)?;
    let factor = usize::from(scale.get());
    let mut row = vec![0u8; row_len];
    let source_width = usize::try_from(image.width).map_err(|_| ExportError::TooLarge)?;
    for cells in image.pixels.chunks_exact(source_width.max(1)) {
        for (cell, block) in cells.iter().zip(row.chunks_exact_mut(factor * CHANNELS)) {
            let rgba = cell.map_or([0; CHANNELS], |c| [c.r, c.g, c.b, 255]);
            for pixel in block.as_chunks_mut::<CHANNELS>().0 {
                *pixel = rgba;
            }
        }
        for _ in 0..factor {
            stream.write_all(&row)?;
        }
    }
    stream.finish().map_err(encoding)?;
    png.finish().map_err(encoding)
}
