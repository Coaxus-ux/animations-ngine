use std::collections::{BTreeMap, BTreeSet};

use px_core::{Paint, Rgb};
use px_ui::{Id, UiCanvas, UiState};

/// Identifies one of the app's windows to the adapter that shows it.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WindowId(pub u32);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PanelKind {
    Layers,
    Palette,
    Timeline,
    Inspector,
    Code,
}

impl PanelKind {
    pub const ALL: [Self; 5] = [
        Self::Layers,
        Self::Palette,
        Self::Timeline,
        Self::Inspector,
        Self::Code,
    ];

    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Layers => "Layers",
            Self::Palette => "Palette",
            Self::Timeline => "Timeline",
            Self::Inspector => "Inspector",
            Self::Code => "Code",
        }
    }

    /// Its entry in the Window menu.
    #[must_use]
    pub const fn window_label(self) -> &'static str {
        match self {
            Self::Layers => "Layers in their own window",
            Self::Palette => "Palette in its own window",
            Self::Timeline => "Timeline in its own window",
            Self::Inspector => "Inspector in its own window",
            Self::Code => "Code in its own window",
        }
    }

    #[must_use]
    pub fn from_title(title: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.title().eq_ignore_ascii_case(title))
    }
}

/// What a window shows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowKind {
    /// The main window: canvas plus every panel that isn't torn off.
    Main,
    /// Another view of the document's canvas, with its own zoom, frame and layer.
    Canvas,
    /// A panel torn off into its own window.
    Panel(PanelKind),
}

impl WindowKind {
    #[must_use]
    pub const fn has_canvas(self) -> bool {
        matches!(self, Self::Main | Self::Canvas)
    }
}

/// How the code view shares the main window with the canvas (F2 cycles it).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CodeMode {
    #[default]
    Hidden,
    Split,
    Only,
}

/// What one canvas window looks at: per window, so two windows can show different zooms,
/// frames and layers of one document.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ViewState {
    /// Window pixels per canvas pixel; 0 fits the canvas to the view.
    pub zoom: i32,
    /// Where the canvas's top-left is, in window pixels from the view's top-left.
    pub pan: Option<(i32, i32)>,
    pub frame: u32,
    /// Index into the document's layers.
    pub layer: Option<usize>,
    /// Names of layers hidden in this window.
    pub hidden: BTreeSet<String>,
    pub onion: bool,
    pub grid: bool,
    pub code: CodeMode,
}

/// The code view's own text while it is being typed (pushed to the document after a pause).
#[derive(Clone, Debug, Default)]
pub struct CodeBuffer {
    pub text: String,
    /// Cursor as a character index into `text`.
    pub cursor: usize,
    pub scroll: usize,
    /// How far long lines are scrolled sideways, in UI pixels.
    pub scroll_x: i32,
    /// The document's text when `text` was last in step with it: the document changed
    /// elsewhere (undo, a brush stroke) when its text no longer equals this.
    pub base: String,
    /// When the text was last typed into, in input milliseconds.
    pub edited_at: Option<u64>,
    /// The picked shape the cursor last jumped to.
    pub jumped: Option<usize>,
}

/// A palette color being edited in the color picker.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Picker {
    pub paint: Paint,
    pub color: Rgb,
}

pub struct Window {
    pub id: WindowId,
    pub kind: WindowKind,
    pub view: ViewState,
    pub ui: UiState,
    pub canvas: UiCanvas,
    /// Window pixels as last drawn (`0x00RRGGBB`).
    pub pixels: Vec<u32>,
    pub size: (usize, usize),
    /// Where the canvas view was last drawn, in window pixels.
    pub area: Option<px_ui::Rect>,
    /// The pointer last frame, in window pixels.
    pub last_pointer: Option<(i32, i32)>,
    /// Panels of the main window torn off into their own windows.
    pub popped: BTreeSet<PanelKind>,
    pub code: CodeBuffer,
    pub picker: Option<Picker>,
    /// Text being typed into fields, by field.
    pub fields: BTreeMap<Id, String>,
    /// A layer being renamed, and the name so far.
    pub renaming: Option<usize>,
    /// The help screen is showing.
    pub help: bool,
    /// The last widget clicked and when, to spot double-clicks.
    pub last_click: Option<(Id, u64)>,
    /// What was drawn last: (document revision, input time bucket), to skip idle redraws.
    pub drawn: Option<(u64, u64)>,
}

impl Window {
    #[must_use]
    pub fn new(id: WindowId, kind: WindowKind, background: Paint) -> Self {
        Self {
            id,
            kind,
            view: ViewState::default(),
            ui: UiState::default(),
            canvas: UiCanvas::new(1, 1, background),
            pixels: Vec::new(),
            size: (0, 0),
            area: None,
            last_pointer: None,
            popped: BTreeSet::new(),
            code: CodeBuffer::default(),
            picker: None,
            fields: BTreeMap::new(),
            renaming: None,
            last_click: None,
            help: false,
            drawn: None,
        }
    }
}

/// What the app asks its adapter to do with windows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    Open {
        id: WindowId,
        title: String,
        width: usize,
        height: usize,
    },
    Close {
        id: WindowId,
    },
    Title {
        id: WindowId,
        title: String,
    },
}
