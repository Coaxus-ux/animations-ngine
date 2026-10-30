//! The workspace: open documents, windows and their views, intents from views applied to
//! the domain, and requests to the adapter for windows to open or close.

mod apply;
mod document_ops;
mod editing;
mod frame;
mod intent;
mod layout;
mod selection_ops;
mod shortcuts;
mod stroke;
mod timeline_ops;
mod view_ops;
mod window;

use std::collections::BTreeMap;
use std::path::Path;

pub use editing::Editing;
pub use intent::{Button, Intent};
pub use window::{
    CodeBuffer, CodeMode, PanelKind, Picker, Request, ViewState, Window, WindowId, WindowKind,
};

use crate::assets::{Assets, ThemeChoice};
use crate::domain::Document;
use crate::ports::FileStore;
use crate::views::RenderCache;
use stroke::Stroke;

/// Window widths (in window pixels) from which the automatic UI size grows a step.
const MEDIUM_FROM: usize = 860;
const LARGE_FROM: usize = 2200;
const MAIN_SIZE: (usize, usize) = (1280, 800);
const CANVAS_SIZE: (usize, usize) = (640, 480);
const PANEL_SIZE: (usize, usize) = (320, 420);

pub struct App {
    assets: Assets,
    theme: ThemeChoice,
    document: Option<Document>,
    editing: Editing,
    stroke: Option<Stroke>,
    windows: Vec<Window>,
    caches: BTreeMap<WindowId, RenderCache>,
    requests: Vec<Request>,
    next_id: u32,
    /// The canvas window panels act on: the last one used.
    focused: WindowId,
    /// Window pixels per UI pixel, or `None` to choose by window size.
    scale: Option<i32>,
    quit: bool,
}

impl App {
    /// # Errors
    /// When the editor's own assets fail to load.
    pub fn new(theme: ThemeChoice) -> Result<Self, String> {
        let mut app = Self {
            assets: Assets::load(theme)?,
            theme,
            document: None,
            editing: Editing::default(),
            stroke: None,
            windows: Vec::new(),
            caches: BTreeMap::new(),
            requests: Vec::new(),
            next_id: 0,
            focused: WindowId(0),
            scale: None,
            quit: false,
        };
        let main = app.open_window(WindowKind::Main);
        app.focused = main;
        Ok(app)
    }

    /// Opens a scene file in the main window, creating it from a blank template if it
    /// doesn't exist and `create` gives a size.
    ///
    /// # Errors
    /// A readable reason when it can't be read or created.
    pub fn open(
        &mut self,
        path: &Path,
        create: Option<(u16, u16)>,
        store: &mut dyn FileStore,
    ) -> Result<(), String> {
        let document = match (store.read(path), create) {
            (Ok(_), _) => Document::open(path, store)?,
            (Err(_), Some((width, height))) => {
                let name = path.file_stem().map_or_else(
                    || "untitled".to_owned(),
                    |stem| stem.to_string_lossy().into_owned(),
                );
                let text = Document::blank_text(&name, width, height);
                store.write(path, &text)?;
                Document::open(path, store)?
            }
            (Err(error), None) => return Err(error),
        };
        self.editing = Editing {
            primary: first_paint(&document),
            ..Editing::default()
        };
        if let Some(scene) = document.scene() {
            let size = scene.size();
            self.editing.selection =
                crate::domain::Selection::none(i32::from(size.width()), i32::from(size.height()));
        }
        let first_layer = document
            .layers()
            .iter()
            .position(crate::domain::Layer::paintable);
        for window in &mut self.windows {
            window.view.layer = first_layer;
        }
        self.document = Some(document);
        self.retitle();
        Ok(())
    }

    #[must_use]
    pub const fn document(&self) -> Option<&Document> {
        self.document.as_ref()
    }

    #[must_use]
    pub const fn editing(&self) -> &Editing {
        &self.editing
    }

    #[must_use]
    pub fn window(&self, id: WindowId) -> Option<&Window> {
        self.windows.iter().find(|window| window.id == id)
    }

    pub fn window_ids(&self) -> impl Iterator<Item = WindowId> + '_ {
        self.windows.iter().map(|window| window.id)
    }

    /// Window pixels per UI pixel for a window `width` window pixels wide.
    #[must_use]
    pub const fn scale_for(&self, width: usize) -> i32 {
        match self.scale {
            Some(scale) => scale,
            None if width >= LARGE_FROM => 3,
            None if width >= MEDIUM_FROM => 2,
            None => 1,
        }
    }

    /// Requests for the adapter since the last call: windows to open, close or retitle.
    pub fn take_requests(&mut self) -> Vec<Request> {
        core::mem::take(&mut self.requests)
    }

    /// The adapter closed window `id` (the user closed it): a torn-off panel docks back,
    /// and closing the main window quits.
    pub fn window_closed(&mut self, id: WindowId) {
        let Some(index) = self.windows.iter().position(|window| window.id == id) else {
            return;
        };
        let window = self.windows.remove(index);
        self.caches.remove(&id);
        match window.kind {
            WindowKind::Main => self.quit = true,
            WindowKind::Panel(panel) => {
                if let Some(main) = self.main_mut() {
                    main.popped.remove(&panel);
                }
            }
            WindowKind::Canvas => {}
        }
        if self.focused == id {
            self.focused = self.windows.first().map_or(WindowId(0), |window| window.id);
        }
    }

    #[must_use]
    pub const fn should_quit(&self) -> bool {
        self.quit
    }

    fn open_window(&mut self, kind: WindowKind) -> WindowId {
        let id = WindowId(self.next_id);
        self.next_id += 1;
        let mut window = Window::new(id, kind, self.assets.theme.roles.window);
        if let Some(focused) = self.windows.iter().find(|other| other.id == self.focused) {
            window.view = ViewState {
                code: CodeMode::Hidden,
                ..focused.view.clone()
            };
        }
        let (width, height) = match kind {
            WindowKind::Main => MAIN_SIZE,
            WindowKind::Canvas => CANVAS_SIZE,
            WindowKind::Panel(_) => PANEL_SIZE,
        };
        self.windows.push(window);
        self.requests.push(Request::Open {
            id,
            title: self.title_for(kind),
            width,
            height,
        });
        id
    }

    fn main_mut(&mut self) -> Option<&mut Window> {
        self.windows
            .iter_mut()
            .find(|window| window.kind == WindowKind::Main)
    }

    fn title_for(&self, kind: WindowKind) -> String {
        let name = self.document.as_ref().map_or_else(
            || "px editor".to_owned(),
            |document| {
                let file = document
                    .path()
                    .file_name()
                    .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
                let dirty = if document.is_dirty() { " *" } else { "" };
                format!("{file}{dirty}")
            },
        );
        match kind {
            WindowKind::Main => format!("{name} - px editor"),
            WindowKind::Canvas => format!("{name} - view"),
            WindowKind::Panel(panel) => format!("{} - {name}", panel.title()),
        }
    }

    fn retitle(&mut self) {
        let titles: Vec<(WindowId, String)> = self
            .windows
            .iter()
            .map(|window| (window.id, self.title_for(window.kind)))
            .collect();
        for (id, title) in titles {
            self.requests.push(Request::Title { id, title });
        }
    }

    fn reload_assets(&mut self) {
        if let Ok(assets) = Assets::load(self.theme) {
            self.assets = assets;
        }
    }
}

/// The first paint of the document's palette, to start drawing with.
fn first_paint(document: &Document) -> Option<px_core::Paint> {
    let scene = document.scene()?;
    let names = &document.loaded()?.paint_names;
    let (_, ramp) = names.ramps().next()?;
    let tone = scene.palette().ramp(ramp).ok()?.len().saturating_sub(1);
    scene.palette().paint(ramp, px_core::Tone::new(tone)).ok()
}
