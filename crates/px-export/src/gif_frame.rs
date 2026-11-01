use std::borrow::Cow;

use px_core::{FrameRate, Paint, Palette, PaletteError, Rgb, Size};

use crate::color_slots::{ColorSlots, GIF_MAX_COLORS};
use crate::error::ExportError;
use crate::resolve::{scaled_len, upscale};
use crate::scale::Scale;

/// One GIF frame, already LZW-compressed: frames can be encoded on any thread and written
/// in order afterwards.
pub struct EncodedFrame(pub(crate) gif::Frame<'static>);

/// Buffers one encoding thread reuses across frames.
#[derive(Default)]
pub struct GifScratch {
    native: Vec<u8>,
    scaled: Vec<u8>,
    slots: ColorSlots,
}

/// Encodes frames as deltas: only the box that changed since the previous frame is stored,
/// and the unchanged pixels inside it are transparent when the color table has room.
#[derive(Clone, Debug)]
pub struct GifFrameEncoder {
    size: Size,
    scale: Scale,
    delay: u16,
    /// The global table's ramps; empty when every frame carries its own table.
    ramp_spans: Vec<RampSpan>,
    transparent: Option<u8>,
}

#[derive(Clone, Copy, Debug)]
struct RampSpan {
    start: u8,
    len: u8,
}

#[derive(Clone, Copy, Debug)]
struct Rect {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

impl GifFrameEncoder {
    /// The encoder and the global color table (the palette's ramps back to back, plus a
    /// transparent slot when it fits).
    pub(crate) fn with_palette(
        size: Size,
        scale: Scale,
        fps: FrameRate,
        palette: &Palette,
    ) -> Result<(Self, Vec<u8>), ExportError> {
        let count: usize = palette.ramps().iter().map(|ramp| ramp.colors().len()).sum();
        let too_many = || ExportError::TooManyColors {
            count,
            max: GIF_MAX_COLORS,
        };
        if count > GIF_MAX_COLORS {
            return Err(too_many());
        }
        let mut table = Vec::with_capacity((count + 1) * 3);
        let mut ramp_spans = Vec::with_capacity(palette.ramps().len());
        for ramp in palette.ramps() {
            let start = u8::try_from(table.len() / 3).map_err(|_| too_many())?;
            ramp_spans.push(RampSpan {
                start,
                len: ramp.len(),
            });
            for color in ramp.colors() {
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
        Ok((encoder, table))
    }

    pub(crate) fn with_frame_palettes(size: Size, scale: Scale, fps: FrameRate) -> Self {
        Self {
            size,
            scale,
            // GIF delays are whole centiseconds: 12 fps plays at 8 cs (≈ 12.5 fps).
            delay: u16::try_from(fps.period(100)).unwrap_or(u16::MAX),
            ramp_spans: Vec::new(),
            transparent: None,
        }
    }

    pub(crate) fn size(&self) -> Size {
        self.size
    }

    pub(crate) fn scale(&self) -> Scale {
        self.scale
    }

    /// A frame of palette paints, indexed into the global table.
    pub fn encode(
        &self,
        cells: &[Paint],
        previous: Option<&[Paint]>,
        scratch: &mut GifScratch,
    ) -> Result<EncodedFrame, ExportError> {
        let previous = self.check(cells, previous)?;
        let rect = changed(self.size, cells, previous);
        let transparent = previous.and(self.transparent);
        scratch.native.clear();
        for (cell, before) in pixels(self.size, rect, cells, previous) {
            scratch.native.push(match transparent {
                Some(slot) if before == Some(cell) => slot,
                _ => table_index(&self.ramp_spans, cell)?,
            });
        }
        self.finish(rect, transparent, None, scratch)
    }

    /// A frame of exact colors with its own table of the colors it stores (at most 256).
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
        self.finish(rect, transparent, Some(table), scratch)
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
        let mut draft = self.frame(rect, transparent, None, Cow::Borrowed(&scratch.scaled))?;
        draft.make_lzw_pre_encoded();
        let lzw = Cow::Owned(draft.buffer.into_owned());
        Ok(EncodedFrame(self.frame(rect, transparent, palette, lzw)?))
    }

    fn frame<'b>(
        &self,
        rect: Rect,
        transparent: Option<u8>,
        palette: Option<Vec<u8>>,
        buffer: Cow<'b, [u8]>,
    ) -> Result<gif::Frame<'b>, ExportError> {
        let scaled = |value: u16| self.scale.apply(value).ok_or(ExportError::TooLarge);
        Ok(gif::Frame {
            left: scaled(rect.x)?,
            top: scaled(rect.y)?,
            width: scaled(rect.width)?,
            height: scaled(rect.height)?,
            delay: self.delay,
            transparent,
            palette,
            buffer,
            ..gif::Frame::default()
        })
    }
}

/// The smallest box holding every cell that differs from `previous`; the whole canvas
/// without one, and a single cell when nothing changed (a GIF frame cannot be empty).
fn changed<T: PartialEq>(size: Size, current: &[T], previous: Option<&[T]>) -> Rect {
    let (width, height) = (size.width(), size.height());
    let full = Rect {
        x: 0,
        y: 0,
        width,
        height,
    };
    let Some(previous) = previous else {
        return full;
    };
    let columns = usize::from(width);
    let rows = current
        .chunks_exact(columns)
        .zip(previous.chunks_exact(columns));
    let mut bounds: Option<[usize; 4]> = None;
    for (y, (now, before)) in rows.enumerate() {
        if now == before {
            continue;
        }
        let differs = |(a, b): (&T, &T)| a != b;
        let first = now.iter().zip(before).position(differs).unwrap_or(0);
        let last = columns
            - 1
            - now
                .iter()
                .rev()
                .zip(before.iter().rev())
                .position(differs)
                .unwrap_or(0);
        let [x0, y0, x1, _] = bounds.unwrap_or([first, y, last, y]);
        bounds = Some([x0.min(first), y0, x1.max(last), y]);
    }
    let Some([x0, y0, x1, y1]) = bounds else {
        return Rect {
            width: 1,
            height: 1,
            ..full
        };
    };
    let narrow = |value: usize| u16::try_from(value).unwrap_or(u16::MAX);
    Rect {
        x: narrow(x0),
        y: narrow(y0),
        width: narrow(x1 - x0 + 1),
        height: narrow(y1 - y0 + 1),
    }
}

/// Each cell of `rect` in row order, with the same cell of `previous`.
fn pixels<'a, T: Copy>(
    size: Size,
    rect: Rect,
    current: &'a [T],
    previous: Option<&'a [T]>,
) -> impl Iterator<Item = (T, Option<T>)> + 'a {
    let columns = usize::from(size.width());
    let (x, width) = (usize::from(rect.x), usize::from(rect.width));
    (usize::from(rect.y)..usize::from(rect.y) + usize::from(rect.height)).flat_map(move |y| {
        let start = y * columns + x;
        let now = &current[start..start + width];
        let before = previous.map(|previous| &previous[start..start + width]);
        (0..width).map(move |i| (now[i], before.map(|before| before[i])))
    })
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
