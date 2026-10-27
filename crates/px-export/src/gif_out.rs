use std::borrow::Cow;
use std::io::Write;

use px_core::{CanvasView, FrameRate, Paint, Palette, PaletteError, Size};

use crate::error::ExportError;
use crate::resolve::{indexed_len, upscale};
use crate::scale::Scale;

const GIF_MAX_COLORS: usize = 256;

pub struct GifWriter<W: Write> {
    encoder: gif::Encoder<W>,
    ramp_spans: Vec<RampSpan>,
    size: Size,
    scale: Scale,
    delay: u16,
    indices: Vec<u8>,
}

impl<W: Write> GifWriter<W> {
    pub fn new(
        writer: W,
        size: Size,
        palette: &Palette,
        scale: Scale,
        fps: FrameRate,
    ) -> Result<Self, ExportError> {
        let (table, ramp_spans) = color_table(palette)?;
        let width = scale.apply(size.width()).ok_or(ExportError::TooLarge)?;
        let height = scale.apply(size.height()).ok_or(ExportError::TooLarge)?;
        let encoding = |error: gif::EncodingError| ExportError::Encoding(error.to_string());
        let mut encoder = gif::Encoder::new(writer, width, height, &table).map_err(encoding)?;
        encoder
            .set_repeat(gif::Repeat::Infinite)
            .map_err(encoding)?;
        // GIF delays are whole centiseconds: 12 fps plays at 8 cs (≈ 12.5 fps).
        let delay = (100 + fps.get() / 2) / fps.get();
        Ok(Self {
            encoder,
            ramp_spans,
            size,
            scale,
            delay,
            indices: Vec::new(),
        })
    }

    pub fn add_frame(&mut self, view: &CanvasView<'_>) -> Result<(), ExportError> {
        if view.size() != self.size {
            return Err(ExportError::TooLarge);
        }
        self.indices.resize(indexed_len(view, self.scale, 1)?, 0);
        let spans = &self.ramp_spans;
        upscale(view, self.scale, 1, &mut self.indices, |paint, pixel| {
            pixel[0] = table_index(spans, paint)?;
            Ok(())
        })?;
        let frame = gif::Frame {
            width: self
                .scale
                .apply(self.size.width())
                .ok_or(ExportError::TooLarge)?,
            height: self
                .scale
                .apply(self.size.height())
                .ok_or(ExportError::TooLarge)?,
            delay: self.delay,
            buffer: Cow::Borrowed(&self.indices),
            ..gif::Frame::default()
        };
        self.encoder
            .write_frame(&frame)
            .map_err(|error| ExportError::Encoding(error.to_string()))
    }
}

#[derive(Clone, Copy, Debug)]
struct RampSpan {
    start: u8,
    len: u8,
}

fn color_table(palette: &Palette) -> Result<(Vec<u8>, Vec<RampSpan>), ExportError> {
    let count: usize = palette.ramps().iter().map(|ramp| ramp.colors().len()).sum();
    let too_many = ExportError::TooManyColors {
        count,
        max: GIF_MAX_COLORS,
    };
    if count > GIF_MAX_COLORS {
        return Err(too_many);
    }
    let mut table = Vec::with_capacity(count * 3);
    let mut spans = Vec::with_capacity(palette.ramps().len());
    for ramp in palette.ramps() {
        let Ok(start) = u8::try_from(table.len() / 3) else {
            return Err(too_many);
        };
        spans.push(RampSpan {
            start,
            len: ramp.len(),
        });
        for color in ramp.colors() {
            table.extend_from_slice(&[color.r, color.g, color.b]);
        }
    }
    Ok((table, spans))
}

fn table_index(spans: &[RampSpan], paint: Paint) -> Result<u8, ExportError> {
    let span = spans
        .get(paint.ramp.index())
        .ok_or(PaletteError::UnknownRamp { ramp: paint.ramp })?;
    let tone = paint.tone;
    if tone.index() >= span.len {
        return Err(PaletteError::ToneOutOfRange {
            ramp: paint.ramp,
            tone,
            len: span.len,
        }
        .into());
    }
    // `start + len` never exceeds 256 because the table holds at most 256 colors.
    Ok(span.start + tone.index())
}
