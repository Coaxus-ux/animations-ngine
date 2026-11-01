use std::collections::BTreeSet;

use px_core::{Frame, Paint, Palette};
use px_engine::{Detail, FrameBuffer, Scene};

use crate::app::{DocumentId, ViewState};
use crate::domain::Document;
use crate::domain::visibility::with_hidden_layers;

pub const FRAME_CACHE_BYTES: usize = 320 << 20;

#[derive(Clone, Debug, PartialEq)]
struct SceneKey {
    document: DocumentId,
    revision: u64,
    hidden: BTreeSet<String>,
    detail: Detail,
}

pub struct Image<'a> {
    pub canvas: (i32, i32),
    pub size: (i32, i32),
    pub cells: &'a [Paint],
    pub palette: &'a Palette,
}

#[derive(Default)]
pub struct RenderCache {
    key: Option<SceneKey>,
    scene: Option<Scene>,
    buffer: Option<FrameBuffer>,
    rendered: Option<u32>,
    shown: Option<u32>,
    unpacked: Vec<Paint>,
    frames: Vec<Option<Box<[u16]>>>,
    cached_bytes: usize,
}

impl RenderCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, id: DocumentId, document: &Document, view: &ViewState, playing: bool) {
        let key = SceneKey {
            document: id,
            revision: document.revision(),
            hidden: view.hidden.clone(),
            detail: view.detail,
        };
        if self.key.as_ref() != Some(&key) {
            self.prepare(document, &key);
            self.key = Some(key);
        }
        let Some(scene) = &self.scene else {
            return;
        };
        let frame = view.frame % scene.timing().duration.get().max(1);
        if playing && self.shown == Some(frame) {
            return;
        }
        if playing && self.unpack(frame) {
            self.shown = Some(frame);
            return;
        }
        if self.rendered != Some(frame) {
            self.render(frame);
        }
        self.shown = self.rendered.filter(|rendered| *rendered == frame);
    }

    fn prepare(&mut self, document: &Document, key: &SceneKey) {
        let Some(scene) = document.scene() else {
            return;
        };
        let visible =
            with_hidden_layers(document, scene, &key.hidden).unwrap_or_else(|| scene.clone());
        let detail = if key.detail.scale(visible.size()).is_some() {
            key.detail
        } else {
            Detail::NATIVE
        };
        let Ok(scene) = visible.with_detail(detail) else {
            return;
        };
        let same_size = self
            .scene
            .as_ref()
            .is_some_and(|old| old.size() == scene.size());
        if !same_size {
            self.buffer = Some(FrameBuffer::new(&scene));
        }
        let duration = usize::try_from(scene.timing().duration.get()).unwrap_or(0);
        self.frames.clear();
        self.frames.resize(duration, None);
        self.cached_bytes = 0;
        self.rendered = None;
        self.shown = None;
        self.scene = Some(scene);
    }

    fn render(&mut self, frame: u32) {
        let (Some(scene), Some(buffer)) = (&self.scene, &mut self.buffer) else {
            return;
        };
        if buffer.render(scene, Frame::new(frame)).is_err() {
            self.rendered = None;
            return;
        }
        self.rendered = Some(frame);
        let cells = buffer.cells();
        let bytes = cells.len() * core::mem::size_of::<u16>();
        let slot = usize::try_from(frame)
            .ok()
            .and_then(|index| self.frames.get_mut(index));
        if let Some(slot @ None) = slot
            && self.cached_bytes + bytes <= FRAME_CACHE_BYTES
        {
            *slot = Some(cells.iter().map(|paint| paint.packed()).collect());
            self.cached_bytes += bytes;
        }
    }

    fn unpack(&mut self, frame: u32) -> bool {
        let Some(Some(packed)) = usize::try_from(frame)
            .ok()
            .and_then(|index| self.frames.get(index))
        else {
            return false;
        };
        self.unpacked.clear();
        self.unpacked
            .extend(packed.iter().map(|bits| Paint::unpacked(*bits)));
        true
    }

    #[must_use]
    pub fn image(&self) -> Option<Image<'_>> {
        let scene = self.scene.as_ref()?;
        let shown = self.shown?;
        let cells = if self.rendered == Some(shown) {
            self.buffer.as_ref()?.cells()
        } else {
            &self.unpacked
        };
        let side = |size: px_core::Size| (i32::from(size.width()), i32::from(size.height()));
        Some(Image {
            canvas: side(scene.spec().size),
            size: side(scene.size()),
            cells,
            palette: scene.palette(),
        })
    }

    #[must_use]
    pub fn shape_at(&self, x: i32, y: i32) -> Option<usize> {
        let image = self.image()?;
        if self.shown != self.rendered {
            return None;
        }
        let (x, y) = image.pixel_of(x, y)?;
        self.buffer.as_ref()?.shape_at(x, y)
    }

    #[must_use]
    pub fn paint_at(&self, x: i32, y: i32) -> Option<Paint> {
        let image = self.image()?;
        let (x, y) = image.pixel_of(x, y)?;
        image
            .cells
            .get(usize::try_from(y * image.size.0 + x).ok()?)
            .copied()
    }
}

impl Image<'_> {
    #[must_use]
    pub fn pixel_of(&self, x: i32, y: i32) -> Option<(i32, i32)> {
        let inside = x >= 0 && y >= 0 && x < self.canvas.0 && y < self.canvas.1;
        let center = |at: i32, canvas: i32, size: i32| {
            let at = (2 * i64::from(at) + 1) * i64::from(size) / (2 * i64::from(canvas.max(1)));
            i32::try_from(at).unwrap_or(0)
        };
        inside.then(|| {
            (
                center(x, self.canvas.0, self.size.0),
                center(y, self.canvas.1, self.size.1),
            )
        })
    }
}
