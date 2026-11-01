use std::io::Write;

use px_core::{CanvasView, FrameRate, Paint, Palette, RampId, Rgb, Size};

use crate::error::ExportError;
use crate::gif_frame::{EncodedFrame, GifFrameEncoder, GifScratch};
use crate::scale::Scale;

pub struct GifWriter<W: Write> {
    encoder: gif::Encoder<W>,
    frames: GifFrameEncoder,
    scratch: GifScratch,
    previous_paints: Vec<Paint>,
    previous_colors: Vec<Rgb>,
}

impl<W: Write> GifWriter<W> {
    pub fn new(
        writer: W,
        size: Size,
        palette: &Palette,
        scale: Scale,
        fps: FrameRate,
    ) -> Result<Self, ExportError> {
        let (frames, table) = GifFrameEncoder::with_palette(size, scale, fps, palette)?;
        Self::open(writer, frames, &table)
    }

    pub fn with_key(
        writer: W,
        size: Size,
        palette: &Palette,
        key: RampId,
        scale: Scale,
        fps: FrameRate,
    ) -> Result<Self, ExportError> {
        let (frames, table) = GifFrameEncoder::with_keyed_palette(size, scale, fps, palette, key)?;
        Self::open(writer, frames, &table)
    }

    pub fn with_frame_palettes(
        writer: W,
        size: Size,
        scale: Scale,
        fps: FrameRate,
    ) -> Result<Self, ExportError> {
        Self::open(
            writer,
            GifFrameEncoder::with_frame_palettes(size, scale, fps),
            &[],
        )
    }

    fn open(writer: W, frames: GifFrameEncoder, table: &[u8]) -> Result<Self, ExportError> {
        let size = frames.size();
        let width = frames
            .scale()
            .apply(size.width())
            .ok_or(ExportError::TooLarge)?;
        let height = frames
            .scale()
            .apply(size.height())
            .ok_or(ExportError::TooLarge)?;
        let mut encoder =
            gif::Encoder::new(writer, width, height, table).map_err(|error| encoding(&error))?;
        encoder
            .set_repeat(gif::Repeat::Infinite)
            .map_err(|error| encoding(&error))?;
        Ok(Self {
            encoder,
            frames,
            scratch: GifScratch::default(),
            previous_paints: Vec::new(),
            previous_colors: Vec::new(),
        })
    }

    #[must_use]
    pub fn frame_encoder(&self) -> &GifFrameEncoder {
        &self.frames
    }

    pub fn write(&mut self, frame: &EncodedFrame) -> Result<(), ExportError> {
        self.encoder
            .write_lzw_pre_encoded_frame(&frame.0)
            .map_err(|error| encoding(&error))
    }

    pub fn add_frame(&mut self, view: &CanvasView<'_>) -> Result<(), ExportError> {
        if view.size() != self.frames.size() {
            return Err(ExportError::TooLarge);
        }
        let previous = (!self.previous_paints.is_empty()).then_some(&self.previous_paints[..]);
        let frame = self
            .frames
            .encode(view.cells(), previous, &mut self.scratch)?;
        self.write(&frame)?;
        self.previous_paints.clear();
        self.previous_paints.extend_from_slice(view.cells());
        Ok(())
    }

    pub fn add_color_frame(&mut self, colors: &[Rgb]) -> Result<(), ExportError> {
        let previous = (!self.previous_colors.is_empty()).then_some(&self.previous_colors[..]);
        let frame = self
            .frames
            .encode_colors(colors, previous, &mut self.scratch)?;
        self.write(&frame)?;
        self.previous_colors.clear();
        self.previous_colors.extend_from_slice(colors);
        Ok(())
    }
}

fn encoding(error: &gif::EncodingError) -> ExportError {
    ExportError::Encoding(error.to_string())
}
