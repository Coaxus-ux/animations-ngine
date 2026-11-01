use std::borrow::Cow;

use px_core::{FrameRate, Paint, Palette, PaletteError, RampId, Rgb, Size};

use crate::color_slots::{ColorSlots, GIF_MAX_COLORS};
use crate::error::ExportError;
use crate::gif_rect::{Rect, changed, pixels};
use crate::resolve::{scaled_len, upscale};
use crate::scale::Scale;

pub struct EncodedFrame(pub(crate) gif::Frame<'static>);

#[derive(Default)]
pub struct GifScratch {
    native: Vec<u8>,
    scaled: Vec<u8>,
    slots: ColorSlots,
}

#[derive(Clone, Debug)]
pub struct GifFrameEncoder {
    size: Size,
    scale: Scale,
    delay: u16,
    ramp_spans: Vec<RampSpan>,
    transparent: Option<u8>,
    key: Option<(RampId, u8)>,
}

#[derive(Clone, Copy, Debug)]
struct RampSpan {
    start: u8,
    len: u8,
}

impl GifFrameEncoder {
    pub(crate) fn with_palette(
        size: Size,
        scale: Scale,
        fps: FrameRate,
        palette: &Palette,
    ) -> Result<(Self, Vec<u8>), ExportError> {
        Self::build(size, scale, fps, palette, None)
    }

    pub(crate) fn with_keyed_palette(
        size: Size,
        scale: Scale,
        fps: FrameRate,
        palette: &Palette,
        key: RampId,
    ) -> Result<(Self, Vec<u8>), ExportError> {
        Self::build(size, scale, fps, palette, Some(key))
    }

    fn build(
        size: Size,
        scale: Scale,
        fps: FrameRate,
        palette: &Palette,
        key: Option<RampId>,
    ) -> Result<(Self, Vec<u8>), ExportError> {
        let kept = |index: usize| key.is_none_or(|key| key.index() != index);
        let count: usize = palette
            .ramps()
            .iter()
            .enumerate()
            .filter(|(index, _)| kept(*index))
            .map(|(_, ramp)| ramp.colors().len())
            .sum();
        let max = GIF_MAX_COLORS - usize::from(key.is_some());
        let too_many = || ExportError::TooManyColors { count, max };
        if count > max {
            return Err(too_many());
        }
        let mut table = Vec::with_capacity((count + 1) * 3);
        let mut ramp_spans = Vec::with_capacity(palette.ramps().len());
        for (index, ramp) in palette.ramps().iter().enumerate() {
            let start = u8::try_from(table.len() / 3).map_err(|_| too_many())?;
            let len = if kept(index) { ramp.len() } else { 0 };
            ramp_spans.push(RampSpan { start, len });
            for color in ramp.colors().iter().take(usize::from(len)) {
                table.extend_from_slice(&[color.r, color.g, color.b]);
            }
        }
        let transparent = u8::try_from(count).ok();
        if transparent.is_some() {
            table.extend_from_slice(&[0, 0, 0]);
        }
        let mut encoder = Self::with_frame_palettes(size, scale, fps);
        encoder.ramp_spans = ramp_spans;
        encoder.transparent = transparent;
        encoder.key = key.zip(transparent);
        Ok((encoder, table))
    }

    pub(crate) fn with_frame_palettes(size: Size, scale: Scale, fps: FrameRate) -> Self {
        Self {
            size,
            scale,
            delay: u16::try_from(fps.period(100)).unwrap_or(u16::MAX),
            ramp_spans: Vec::new(),
            transparent: None,
            key: None,
        }
    }

    pub(crate) fn size(&self) -> Size {
        self.size
    }

    pub(crate) fn scale(&self) -> Scale {
        self.scale
    }

    pub fn encode(
        &self,
        cells: &[Paint],
        previous: Option<&[Paint]>,
        scratch: &mut GifScratch,
    ) -> Result<EncodedFrame, ExportError> {
        let previous = self.check(cells, previous)?;
        if let Some((key, slot)) = self.key {
            return self.encode_keyed(cells, key, slot, scratch);
        }
        let rect = changed(self.size, cells, previous);
        let transparent = previous.and(self.transparent);
        scratch.native.clear();
        for (cell, before) in pixels(self.size, rect, cells, previous) {
            scratch.native.push(match transparent {
                Some(slot) if before == Some(cell) => slot,
                _ => table_index(&self.ramp_spans, cell)?,
            });
        }
        self.finish(rect, transparent, None, gif::DisposalMethod::Keep, scratch)
    }

    fn encode_keyed(
        &self,
        cells: &[Paint],
        key: RampId,
        slot: u8,
        scratch: &mut GifScratch,
    ) -> Result<EncodedFrame, ExportError> {
        let rect = changed(self.size, cells, None);
        scratch.native.clear();
        for (cell, _) in pixels(self.size, rect, cells, None) {
            scratch.native.push(if cell.ramp == key {
                slot
            } else {
                table_index(&self.ramp_spans, cell)?
            });
        }
        self.finish(
            rect,
            Some(slot),
            None,
            gif::DisposalMethod::Background,
            scratch,
        )
    }

    pub fn encode_colors(
        &self,
        colors: &[Rgb],
        previous: Option<&[Rgb]>,
        scratch: &mut GifScratch,
    ) -> Result<EncodedFrame, ExportError> {
        let previous = self.check(colors, previous)?;
        let rect = changed(self.size, colors, previous);
        let slots = &mut scratch.slots;
        slots.clear();
        let mut transparent = None;
        if previous.is_some() {
            for (color, before) in pixels(self.size, rect, colors, previous) {
                if before != Some(color) {
                    slots.insert(color)?;
                }
            }
            transparent = slots.reserve();
            if transparent.is_none() {
                slots.clear();
            }
        }
        if transparent.is_none() {
            for (color, _) in pixels(self.size, rect, colors, None) {
                slots.insert(color)?;
            }
        }
        scratch.native.clear();
        for (color, before) in pixels(self.size, rect, colors, previous) {
            scratch.native.push(match transparent {
                Some(slot) if before == Some(color) => slot,
                _ => slots.get(color).unwrap_or(0),
            });
        }
        let table = slots.table().to_vec();
        self.finish(
            rect,
            transparent,
            Some(table),
            gif::DisposalMethod::Keep,
            scratch,
        )
    }

    fn check<'p, T>(
        &self,
        current: &[T],
        previous: Option<&'p [T]>,
    ) -> Result<Option<&'p [T]>, ExportError> {
        let count = self.size.cell_count();
        if current.len() != count || previous.is_some_and(|previous| previous.len() != count) {
            return Err(ExportError::TooLarge);
        }
        Ok(previous)
    }

    fn finish(
        &self,
        rect: Rect,
        transparent: Option<u8>,
        palette: Option<Vec<u8>>,
        dispose: gif::DisposalMethod,
        scratch: &mut GifScratch,
    ) -> Result<EncodedFrame, ExportError> {
        let size = Size::new(rect.width, rect.height).map_err(ExportError::Canvas)?;
        scratch.scaled.resize(scaled_len(size, self.scale, 1)?, 0);
        upscale(
            size,
            &scratch.native,
            self.scale,
            1,
            &mut scratch.scaled,
            |index, pixel| {
                pixel[0] = index;
                Ok(())
            },
        )?;
        let mut draft = self.frame(
            rect,
            transparent,
            None,
            dispose,
            Cow::Borrowed(&scratch.scaled),
        )?;
        draft.make_lzw_pre_encoded();
        let lzw = Cow::Owned(draft.buffer.into_owned());
        Ok(EncodedFrame(self.frame(
            rect,
            transparent,
            palette,
            dispose,
            lzw,
        )?))
    }

    fn frame<'b>(
        &self,
        rect: Rect,
        transparent: Option<u8>,
        palette: Option<Vec<u8>>,
        dispose: gif::DisposalMethod,
        buffer: Cow<'b, [u8]>,
    ) -> Result<gif::Frame<'b>, ExportError> {
        let scaled = |value: u16| self.scale.apply(value).ok_or(ExportError::TooLarge);
        Ok(gif::Frame {
            left: scaled(rect.x)?,
            top: scaled(rect.y)?,
            width: scaled(rect.width)?,
            height: scaled(rect.height)?,
            delay: self.delay,
            dispose,
            transparent,
            palette,
            buffer,
            ..gif::Frame::default()
        })
    }
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
    Ok(span.start + tone.index())
}
