use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use px_core::{Paint, Rgb};
use px_format::edit::NodePath;
use px_ui::{Id, UiCanvas, UiState};

use super::dialog::{Dialog, PickPurpose};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WindowId(pub u32);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PanelKind {
    Layers,
    Palette,
    Timeline,
    Inspector,
    Code,
    Elements,
}

impl PanelKind {
    pub const ALL: [Self; 6] = [
        Self::Layers,
        Self::Palette,
        Self::Timeline,
        Self::Inspector,
        Self::Code,
        Self::Elements,
    ];

    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Layers => "Layers",
            Self::Palette => "Palette",
            Self::Timeline => "Timeline",
            Self::Inspector => "Inspector",
            Self::Code => "Code",
            Self::Elements => "Elements",
        }
    }

    #[must_use]
    pub const fn window_label(self) -> &'static str {
        match self {
            Self::Layers => "Layers in their own window",
            Self::Palette => "Palette in its own window",
            Self::Timeline => "Timeline in its own window",
            Self::Inspector => "Inspector in its own window",
            Self::Code => "Code in its own window",
            Self::Elements => "Elements in their own window",
        }
    }

    #[must_use]
    pub fn from_title(title: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.title().eq_ignore_ascii_case(title))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowKind {
    Main,
    Canvas,
    Panel(PanelKind),
}

impl WindowKind {
    #[must_use]
    pub const fn has_canvas(self) -> bool {
        matches!(self, Self::Main | Self::Canvas)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CodeMode {
    #[default]
    Hidden,
    Split,
    Only,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ViewState {
    pub zoom: i32,
    pub pan: Option<(i32, i32)>,
    pub frame: u32,
    pub layer: Option<usize>,
    pub hidden: BTreeSet<String>,
    pub onion: bool,
    pub grid: bool,
    pub code: CodeMode,
}

#[derive(Clone, Debug, Default)]
pub struct CodeBuffer {
    pub text: String,
    pub cursor: usize,
    pub scroll: usize,
    pub scroll_x: i32,
    pub base: String,
    pub edited_at: Option<u64>,
    pub jumped: Option<usize>,
}

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
    pub pixels: Vec<u32>,
    pub size: (usize, usize),
    pub area: Option<px_ui::Rect>,
    pub last_pointer: Option<(i32, i32)>,
    pub popped: BTreeSet<PanelKind>,
    pub code: CodeBuffer,
    pub picker: Option<Picker>,
    pub fields: BTreeMap<Id, String>,
    pub renaming: Option<usize>,
    pub help: bool,
    pub dialog: Option<Dialog>,
    pub selected: Option<NodePath>,
    pub palette_edit: bool,
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
            help: false,
            dialog: None,
            selected: None,
            palette_edit: false,
            drawn: None,
        }
    }
}

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
    PickFile {
        window: WindowId,
        purpose: PickPurpose,
        start: Option<PathBuf>,
        name: Option<String>,
        filters: &'static [(&'static str, &'static [&'static str])],
    },
}
