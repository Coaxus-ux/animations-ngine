use anyhow::{Context, Result};
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use px_core::{CanvasView, Palette, Size};
use px_export::{Scale, indexed_len, resolve_rgb};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Control {
    TogglePause,
    StepBack,
    StepForward,
    Restart,
    Quit,
}

const BINDINGS: [(Key, Control); 7] = [
    (Key::Space, Control::TogglePause),
    (Key::Left, Control::StepBack),
    (Key::Right, Control::StepForward),
    (Key::R, Control::Restart),
    (Key::Escape, Control::Quit),
    (Key::Q, Control::Quit),
    (Key::Home, Control::Restart),
];

pub(crate) struct Preview {
    window: Window,
    size: Size,
    scale: Scale,
    rgb: Vec<u8>,
    pixels: Vec<u32>,
}

impl Preview {
    pub(crate) fn open(size: Size, scale: Scale, title: &str) -> Result<Self> {
        let (width, height) = scaled(size, scale);
        let mut window = Window::new(title, width, height, WindowOptions::default())
            .context("cannot open the preview window")?;
        window.set_target_fps(60);
        Ok(Self {
            window,
            size,
            scale,
            rgb: Vec::new(),
            pixels: Vec::new(),
        })
    }

    pub(crate) const fn size(&self) -> Size {
        self.size
    }

    pub(crate) fn is_open(&self) -> bool {
        self.window.is_open()
    }

    pub(crate) fn set_title(&mut self, title: &str) {
        self.window.set_title(title);
    }

    pub(crate) fn controls(&self) -> Vec<Control> {
        BINDINGS
            .iter()
            .filter(|(key, _)| self.window.is_key_pressed(*key, KeyRepeat::Yes))
            .map(|(_, control)| *control)
            .collect()
    }

    pub(crate) fn show(&mut self, view: &CanvasView<'_>, palette: &Palette) -> Result<()> {
        self.rgb.resize(indexed_len(view, self.scale, 3)?, 0);
        resolve_rgb(view, palette, self.scale, &mut self.rgb)?;
        self.pixels.clear();
        let (rgb, _) = self.rgb.as_chunks::<3>();
        self.pixels.extend(
            rgb.iter()
                .map(|[r, g, b]| u32::from_be_bytes([0, *r, *g, *b])),
        );
        self.idle_with_current()
    }

    pub(crate) fn idle_with_current(&mut self) -> Result<()> {
        let (width, height) = scaled(self.size, self.scale);
        if self.pixels.len() == width * height {
            self.window
                .update_with_buffer(&self.pixels, width, height)?;
        } else {
            self.window.update();
        }
        Ok(())
    }
}

fn scaled(size: Size, scale: Scale) -> (usize, usize) {
    let factor = usize::from(scale.get());
    (
        usize::from(size.width()) * factor,
        usize::from(size.height()) * factor,
    )
}
