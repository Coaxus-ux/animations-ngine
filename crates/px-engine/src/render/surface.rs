use px_core::Paint;
use px_raster::Rect;

use super::mask::Mask;
use super::placement::Placed;

pub(crate) const EMISSIVE: u8 = 1;
pub(crate) const CASTS_SHADOW: u8 = 2;

/// The frame being built: paints plus the per-pixel data lighting needs.
pub(crate) struct Surface<'a> {
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) cells: &'a mut [Paint],
    pub(crate) depth: &'a mut [u16],
    pub(crate) flags: &'a mut [u8],
}

impl Surface<'_> {
    pub(crate) fn index(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.width && y < self.height)
            .then(|| usize::try_from(y * self.width + x).unwrap_or(0))
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Region {
    pub(crate) rect: Rect,
    pub(crate) round: bool,
}

impl Region {
    pub(crate) fn contains(self, x: i32, y: i32) -> bool {
        let Rect {
            x: left,
            y: top,
            width,
            height,
        } = self.rect;
        if x < left || y < top || x >= left + width || y >= top + height {
            return false;
        }
        if !self.round {
            return true;
        }
        let (w, h) = (i64::from(width.max(1)), i64::from(height.max(1)));
        let dx = 2 * i64::from(x) + 1 - (2 * i64::from(left) + w);
        let dy = 2 * i64::from(y) + 1 - (2 * i64::from(top) + h);
        dx * dx * h * h + dy * dy * w * w <= (w * h) * (w * h)
    }
}

/// Everything a frame renders into.
pub(crate) struct Target<'b, 'c> {
    pub(crate) surface: &'b mut Surface<'c>,
    pub(crate) mask: &'b mut Mask<'c>,
    pub(crate) placed: &'b mut [Placed],
}
