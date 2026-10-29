use std::ops::Range;

use px_core::Paint;

use super::report::FrameReport;
use super::surface::{Region, Surface};
use super::transform::Transform;

pub(crate) const MAX_NESTED_CLIPS: usize = 8;

/// One pixel a shape wrote: its paint and surface flags (the depth is the shape's own).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Write {
    pub(crate) index: u32,
    pub(crate) paint: Paint,
    pub(crate) flags: u8,
}

/// Everything outside a shape's own values that decides its pixels: where it lands on the
/// canvas (camera, layer, placed instance, anchor) and the regions it is clipped to.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Key {
    transform: Transform,
    pub(crate) clips: [Region; MAX_NESTED_CLIPS],
}

impl Key {
    pub(crate) const fn new(transform: Transform, clips: [Region; MAX_NESTED_CLIPS]) -> Self {
        Self { transform, clips }
    }

    /// Bit-for-bit, so `-0.0` and `0.0` never count as the same placement.
    fn matches(&self, other: &Self) -> bool {
        let bits = |t: &Transform| [t.scale, t.dx, t.dy].map(f64::to_bits);
        bits(&self.transform) == bits(&other.transform)
            && self.transform.flip == other.transform.flip
            && self.clips == other.clips
    }
}

/// The pixels of every shape whose own values never change, captured once when the scene
/// is built. A later frame copies them instead of drawing the shape again whenever the
/// shape lands exactly where it was captured.
///
/// That copy is exact: a timeless shape evaluates the same values in every frame, so with
/// the same `Key` it takes the same branches and writes the same pixels, and stamped shapes
/// never read the surface.
#[derive(Clone, Debug, Default)]
pub(crate) struct Stamps {
    shapes: Vec<Option<Stamp>>,
    writes: Vec<Write>,
}

#[derive(Clone, Debug)]
struct Stamp {
    key: Key,
    writes: Range<usize>,
    non_finite: u32,
}

impl Stamps {
    /// Draws shape `index` from its stamp; false when it has none for `key` and must be drawn.
    pub(crate) fn replay(
        &self,
        index: usize,
        key: &Key,
        surface: &mut Surface<'_>,
        report: &mut FrameReport,
    ) -> bool {
        let Some(Some(stamp)) = self.shapes.get(index) else {
            return false;
        };
        if !stamp.key.matches(key) {
            return false;
        }
        let depth = shape_depth(index);
        for write in &self.writes[stamp.writes.clone()] {
            let at = write.index as usize;
            surface.cells[at] = write.paint;
            surface.depth[at] = depth;
            surface.flags[at] = write.flags;
        }
        for _ in 0..stamp.non_finite {
            report.record_non_finite(index);
        }
        true
    }
}

/// Collects stamps while one frame is drawn.
pub(crate) struct Capture {
    stamps: Stamps,
    open: usize,
}

impl Capture {
    pub(crate) fn new(shapes: usize) -> Self {
        Self {
            stamps: Stamps {
                shapes: vec![None; shapes],
                writes: Vec::new(),
            },
            open: 0,
        }
    }

    /// Where the shape being drawn records its pixels.
    pub(crate) fn writes(&mut self) -> &mut Vec<Write> {
        &mut self.stamps.writes
    }

    /// Ends shape `index`: keeps what it wrote as its stamp for `key`, or drops it.
    pub(crate) fn finish(&mut self, index: usize, key: Option<Key>, non_finite: u32) {
        let end = self.stamps.writes.len();
        match (key, self.stamps.shapes.get_mut(index)) {
            (Some(key), Some(slot)) => {
                *slot = Some(Stamp {
                    key,
                    writes: self.open..end,
                    non_finite,
                });
            }
            _ => self.stamps.writes.truncate(self.open),
        }
        self.open = self.stamps.writes.len();
    }

    /// Ends a shape that was not drawn.
    pub(crate) fn skip(&mut self) {
        self.stamps.writes.truncate(self.open);
    }

    pub(crate) fn into_stamps(self) -> Stamps {
        self.stamps
    }
}

/// Draw order doubles as depth, with 0 for the background.
pub(crate) fn shape_depth(index: usize) -> u16 {
    u16::try_from(index + 1).unwrap_or(u16::MAX)
}
