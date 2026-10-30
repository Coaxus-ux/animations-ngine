use kdl::KdlValue;
use px_core::{Paint, Rgb};
use px_format::edit::NodePath;

use super::window::{CodeMode, PanelKind};
use crate::assets::ThemeChoice;
use crate::tools::ToolKind;

/// Which mouse button drives a canvas stroke: left paints with the primary paint, right
/// with the secondary one (Aseprite's convention).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Button {
    Primary,
    Secondary,
}

/// What a view asks for; the app applies intents after the frame is drawn, so views only
/// read state (D-015).
#[derive(Clone, Debug, PartialEq)]
pub enum Intent {
    Tool(ToolKind),
    Filled(bool),
    PrimaryPaint(Option<Paint>),
    SecondaryPaint(Option<Paint>),
    SwapPaints,
    /// A swatch was clicked: draw with it, or recolor the picked object.
    Swatch(Paint),
    OpenPicker(Paint),
    ClosePicker,
    SetColor {
        paint: Paint,
        color: Rgb,
    },

    SelectLayer(usize),
    ToggleHidden(String),
    ToggleLocked(String),
    AddLayer,
    DeleteLayer(usize),
    MoveLayer {
        from: usize,
        to: usize,
    },
    RenameLayer {
        layer: usize,
        name: String,
    },
    StartRename(usize),
    Opacity {
        layer: usize,
        value: f64,
    },

    Frame(u32),
    /// Moves the frame by this many, wrapping around.
    StepFrame(i32),
    TogglePlay,
    ToggleOnion,
    ToggleGrid,
    NewCel,
    DeleteCel,
    AddFrame,
    DeleteFrame,

    /// Zoom by `steps` around a point of the view (window pixels).
    Zoom {
        steps: i32,
        at: (i32, i32),
    },
    ZoomFit,
    Pan {
        dx: i32,
        dy: i32,
    },
    CanvasPress {
        at: (i32, i32),
        button: Button,
    },
    CanvasDrag {
        at: (i32, i32),
    },
    CanvasRelease {
        at: (i32, i32),
    },

    SetProperty {
        path: NodePath,
        key: String,
        value: KdlValue,
    },
    SetText(String),
    Undo,
    Redo,
    Save,
    CycleCode,

    Copy,
    Cut,
    Paste,
    Flip {
        horizontal: bool,
    },
    DeleteSelection,
    SelectAll,
    Deselect,

    NewWindow,
    PopOut(PanelKind),
    Dock(PanelKind),
    SetTheme(ThemeChoice),
    /// Window pixels per UI pixel, or `None` to follow the window's size.
    SetScale(Option<i32>),
    SetCode(CodeMode),
    ToggleHelp,
    /// The fill bucket fills every matching pixel, not only touching ones.
    GlobalFill(bool),
    /// Gives the picked object the primary paint.
    RecolorPicked,
    /// Opens the color picker on the primary paint.
    EditPrimaryColor,
}
