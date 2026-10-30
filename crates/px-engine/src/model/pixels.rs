use px_core::Paint;

use super::param::Param;

/// Hand-painted pixels (FORMAT appendix A.9): `cels` sorted by `frame`; each shows from its frame
/// until the next one, like cels on an Aseprite timeline.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PixelArt {
    pub x: Param,
    pub y: Param,
    pub cels: Vec<Cel>,
}

/// One painted image: its box (relative to the layer's origin) and a paint, or nothing,
/// per pixel, row by row.
#[derive(Clone, Debug, PartialEq)]
pub struct Cel {
    pub frame: u32,
    pub x: i32,
    pub y: i32,
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<Option<Paint>>,
}

impl PixelArt {
    /// The cel showing at `frame`: the last one starting at or before it.
    #[must_use]
    pub fn cel_at(&self, frame: u32) -> Option<&Cel> {
        self.cels.iter().rev().find(|cel| cel.frame <= frame)
    }

    /// Whether what shows depends on the frame.
    #[must_use]
    pub fn is_animated(&self) -> bool {
        self.cels.iter().any(|cel| cel.frame > 0)
    }

    pub fn paints(&self) -> impl Iterator<Item = Paint> + '_ {
        self.cels
            .iter()
            .flat_map(|cel| cel.pixels.iter().flatten().copied())
    }
}

impl Cel {
    /// An empty cel of the given box.
    #[must_use]
    pub fn empty(frame: u32, x: i32, y: i32, width: u16, height: u16) -> Self {
        Self {
            frame,
            x,
            y,
            width,
            height,
            pixels: vec![None; usize::from(width) * usize::from(height)],
        }
    }

    /// The paint at `(x, y)` in layer coordinates.
    #[must_use]
    pub fn get(&self, x: i32, y: i32) -> Option<Paint> {
        self.index(x, y).and_then(|index| self.pixels[index])
    }

    /// The pixel slot of `(x, y)` in layer coordinates, if it is inside the cel.
    #[must_use]
    pub fn index(&self, x: i32, y: i32) -> Option<usize> {
        let column = usize::try_from(x - self.x).ok()?;
        let row = usize::try_from(y - self.y).ok()?;
        (column < usize::from(self.width) && row < usize::from(self.height))
            .then(|| row * usize::from(self.width) + column)
    }
}
