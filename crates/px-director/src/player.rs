use px_core::{Frame, Paint, Palette, PaletteError, Rgb, Size, hash_bytes, hash_cells, hash_seed};
use px_engine::{FrameBuffer, RenderError};

use crate::sequence::{Moment, Sequence};
use crate::transition::{Side, Target, compose};

/// Renders sequence frames. Two frame buffers are enough: every shot has the sequence's
/// size, and a transition shows two neighbouring shots, which use different buffers.
#[derive(Clone, Debug)]
pub struct Player {
    buffers: [FrameBuffer; 2],
    paints: Vec<Paint>,
    shots: Vec<u8>,
}

/// A composed frame: each pixel's paint and the shot (so the palette) it comes from.
#[derive(Clone, Copy, Debug)]
pub struct SequenceView<'a> {
    sequence: &'a Sequence,
    paints: &'a [Paint],
    shots: &'a [u8],
}

impl Player {
    #[must_use]
    pub fn new(sequence: &Sequence) -> Self {
        let first = &sequence.shots()[0].scene;
        let count = sequence.size().cell_count();
        Self {
            buffers: [FrameBuffer::new(first), FrameBuffer::new(first)],
            paints: vec![first.background(); count],
            shots: vec![0; count],
        }
    }

    pub fn render<'a>(
        &'a mut self,
        sequence: &'a Sequence,
        frame: Frame,
    ) -> Result<SequenceView<'a>, RenderError> {
        let shots = sequence.shots();
        match sequence.moment(frame) {
            Moment::Shot { shot, frame } => {
                let view = self.buffers[shot % 2].render(&shots[shot].scene, frame)?;
                self.paints.copy_from_slice(view.cells());
                self.shots.fill(slot(shot));
            }
            Moment::Between {
                from,
                to,
                kind,
                progress,
            } => {
                let [even, odd] = &mut self.buffers;
                let (first, second) = if from.0 % 2 == 0 {
                    (even, odd)
                } else {
                    (odd, even)
                };
                let side = |buffer: &'a mut FrameBuffer, (shot, frame): (usize, Frame)| {
                    let scene = &shots[shot].scene;
                    let view = buffer.render(scene, frame)?;
                    Ok::<_, RenderError>(Side {
                        shot: slot(shot),
                        cells: view.cells(),
                        palette: scene.palette(),
                    })
                };
                let sides = [side(first, from)?, side(second, to)?];
                let target = Target {
                    paints: &mut self.paints,
                    shots: &mut self.shots,
                };
                compose(kind, progress, sequence.size(), sides, target);
            }
        }
        Ok(SequenceView {
            sequence,
            paints: &self.paints,
            shots: &self.shots,
        })
    }
}

fn slot(shot: usize) -> u8 {
    u8::try_from(shot).unwrap_or(u8::MAX)
}

impl SequenceView<'_> {
    #[must_use]
    pub fn size(&self) -> Size {
        self.sequence.size()
    }

    #[must_use]
    pub const fn paints(&self) -> &[Paint] {
        self.paints
    }

    /// The shot each pixel comes from, as an index into `Sequence::shots`.
    #[must_use]
    pub const fn shots(&self) -> &[u8] {
        self.shots
    }

    #[must_use]
    pub fn palette(&self, shot: u8) -> &Palette {
        self.sequence.shots()[usize::from(shot)].scene.palette()
    }

    pub fn resolve(&self, index: usize) -> Result<Rgb, PaletteError> {
        self.palette(self.shots[index]).resolve(self.paints[index])
    }

    /// Hash of the indexed frame, palette-independent like `hash_cells`.
    #[must_use]
    pub fn hash(&self) -> u64 {
        hash_seed(hash_cells(self.paints), &[hash_bytes(self.shots)])
    }
}
