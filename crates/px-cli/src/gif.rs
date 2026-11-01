use std::fs::File;
use std::io::BufWriter;
use std::num::NonZeroUsize;
use std::ops::Range;
use std::path::Path;

use anyhow::Result;
use px_core::{Frame, Paint, Rgb};
use px_director::{Player, Sequence};
use px_engine::{FrameBuffer, FrameReport, Scene};
use px_export::{EncodedFrame, ExportError, GifFrameEncoder, GifScratch, GifWriter, Scale};

use crate::commands::create;
use crate::parallel;

pub(crate) fn write_scene(
    scene: &Scene,
    scale: Scale,
    jobs: NonZeroUsize,
    path: &Path,
) -> Result<FrameReport> {
    let timing = scene.timing();
    let file = BufWriter::new(create(path)?);
    let gif = GifWriter::new(file, scene.size(), scene.palette(), scale, timing.fps)?;
    write(gif, timing.duration.get(), jobs, || SceneFrames {
        scene,
        buffer: FrameBuffer::new(scene),
    })
}

pub(crate) fn write_sequence(
    sequence: &Sequence,
    scale: Scale,
    jobs: NonZeroUsize,
    path: &Path,
) -> Result<()> {
    let file = BufWriter::new(create(path)?);
    let gif = GifWriter::with_frame_palettes(file, sequence.size(), scale, sequence.fps())?;
    write(gif, sequence.duration().get(), jobs, || SequenceFrames {
        sequence,
        player: Player::new(sequence),
        colors: Vec::new(),
    })?;
    Ok(())
}

fn write<S: Source>(
    mut gif: GifWriter<BufWriter<File>>,
    frames: u32,
    jobs: NonZeroUsize,
    source: impl Fn() -> S + Sync,
) -> Result<FrameReport> {
    let encoder = gif.frame_encoder().clone();
    let mut total = FrameReport::default();
    parallel::ordered(
        frames,
        jobs,
        || Worker {
            source: source(),
            previous: Vec::new(),
            last: None,
            scratch: GifScratch::default(),
        },
        |worker, range| worker.encode(&encoder, range),
        |(frame, report)| {
            gif.write(&frame)?;
            total.non_finite_values = total
                .non_finite_values
                .saturating_add(report.non_finite_values);
            total.first_non_finite_shape = total
                .first_non_finite_shape
                .or(report.first_non_finite_shape);
            Ok(())
        },
    )?;
    Ok(total)
}

trait Source {
    type Pixel: Copy;

    fn render(&mut self, frame: Frame) -> Result<&[Self::Pixel]>;

    fn encode(
        encoder: &GifFrameEncoder,
        pixels: &[Self::Pixel],
        previous: Option<&[Self::Pixel]>,
        scratch: &mut GifScratch,
    ) -> Result<EncodedFrame, ExportError>;

    fn report(&self) -> FrameReport;
}

struct Worker<S: Source> {
    source: S,
    previous: Vec<S::Pixel>,
    last: Option<u32>,
    scratch: GifScratch,
}

impl<S: Source> Worker<S> {
    fn encode(
        &mut self,
        encoder: &GifFrameEncoder,
        chunk: Range<u32>,
    ) -> Result<Vec<(EncodedFrame, FrameReport)>> {
        if let Some(before) = chunk.start.checked_sub(1)
            && self.last != Some(before)
        {
            self.remember(before)?;
        }
        let mut frames = Vec::with_capacity(chunk.len());
        for index in chunk {
            let pixels = self.source.render(Frame::new(index))?;
            let previous = (index > 0).then_some(&self.previous[..]);
            let gif_frame = S::encode(encoder, pixels, previous, &mut self.scratch)?;
            self.previous.clear();
            self.previous.extend_from_slice(pixels);
            self.last = Some(index);
            frames.push((gif_frame, self.source.report()));
        }
        Ok(frames)
    }

    fn remember(&mut self, index: u32) -> Result<()> {
        let pixels = self.source.render(Frame::new(index))?;
        self.previous.clear();
        self.previous.extend_from_slice(pixels);
        self.last = Some(index);
        Ok(())
    }
}

struct SceneFrames<'s> {
    scene: &'s Scene,
    buffer: FrameBuffer,
}

impl Source for SceneFrames<'_> {
    type Pixel = Paint;

    fn render(&mut self, frame: Frame) -> Result<&[Paint]> {
        Ok(self.buffer.render(self.scene, frame)?.cells())
    }

    fn encode(
        encoder: &GifFrameEncoder,
        pixels: &[Paint],
        previous: Option<&[Paint]>,
        scratch: &mut GifScratch,
    ) -> Result<EncodedFrame, ExportError> {
        encoder.encode(pixels, previous, scratch)
    }

    fn report(&self) -> FrameReport {
        self.buffer.report()
    }
}

struct SequenceFrames<'s> {
    sequence: &'s Sequence,
    player: Player,
    colors: Vec<Rgb>,
}

impl Source for SequenceFrames<'_> {
    type Pixel = Rgb;

    fn render(&mut self, frame: Frame) -> Result<&[Rgb]> {
        crate::sequence::resolve(&self.player.render(self.sequence, frame)?, &mut self.colors)?;
        Ok(&self.colors)
    }

    fn encode(
        encoder: &GifFrameEncoder,
        pixels: &[Rgb],
        previous: Option<&[Rgb]>,
        scratch: &mut GifScratch,
    ) -> Result<EncodedFrame, ExportError> {
        encoder.encode_colors(pixels, previous, scratch)
    }

    fn report(&self) -> FrameReport {
        FrameReport::default()
    }
}
