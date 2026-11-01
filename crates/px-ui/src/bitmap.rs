use std::collections::BTreeMap;

use px_core::{Frame, Paint};
use px_engine::{FrameBuffer, RenderError, Scene};

/// A small indexed image with transparent pixels: an icon rendered by the engine.
#[derive(Clone, Debug, PartialEq)]
pub struct Bitmap {
    width: i32,
    height: i32,
    cells: Vec<Option<Paint>>,
}

impl Bitmap {
    /// Renders frame 0 of `scene`; pixels left at `transparent` (the scene's background,
    /// usually) become see-through.
    pub fn render(scene: &Scene, transparent: Paint) -> Result<Self, RenderError> {
        let mut buffer = FrameBuffer::new(scene);
        let view = buffer.render(scene, Frame::new(0))?;
        let size = view.size();
        Ok(Self {
            width: i32::from(size.width()),
            height: i32::from(size.height()),
            cells: view
                .cells()
                .iter()
                .map(|paint| (*paint != transparent).then_some(*paint))
                .collect(),
        })
    }

    #[must_use]
    pub const fn width(&self) -> i32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> i32 {
        self.height
    }

    /// Every opaque pixel as (row, column, paint).
    pub fn opaque(&self) -> impl Iterator<Item = (i32, i32, Paint)> + '_ {
        let width = usize::try_from(self.width.max(1)).unwrap_or(1);
        (0i32..)
            .zip(self.cells.chunks(width))
            .flat_map(|(row, cells)| {
                (0i32..)
                    .zip(cells)
                    .filter_map(move |(column, cell)| cell.map(|paint| (row, column, paint)))
            })
    }
}

#[derive(Clone, Debug, Default)]
pub struct Icons {
    icons: BTreeMap<String, Bitmap>,
}

impl Icons {
    pub fn insert(&mut self, name: impl Into<String>, bitmap: Bitmap) {
        self.icons.insert(name.into(), bitmap);
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Bitmap> {
        self.icons.get(name)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.icons.keys().map(String::as_str)
    }
}
