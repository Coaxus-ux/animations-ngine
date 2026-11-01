use std::path::PathBuf;

use kdl::KdlValue;
use px_core::{Paint, Rgb};
use px_format::edit::NodePath;

use super::dialog::{DialogKind, PickPurpose};
use super::window::{CodeMode, PanelKind};
use crate::assets::ThemeChoice;
use crate::domain::{Anchor, ExportJob, NewSpec};
use crate::tools::ToolKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Button {
    Primary,
    Secondary,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Intent {
    Tool(ToolKind),
    Filled(bool),
    PrimaryPaint(Option<Paint>),
    SecondaryPaint(Option<Paint>),
    SwapPaints,
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
    CancelRename,
    Opacity {
        layer: usize,
        value: f64,
    },

    Frame(u32),
    StepFrame(i32),
    TogglePlay,
    ToggleOnion,
    ToggleGrid,
    NewCel,
    DeleteCel,
    AddFrame,
    DeleteFrame,

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
    CopyText(String),
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
    SetScale(Option<i32>),
    SetCode(CodeMode),
    TogglePaletteSide,
    ToggleHelp,
    GlobalFill(bool),
    RecolorPicked,
    EditPrimaryColor,

    ShowDialog(DialogKind),
    CloseDialog,
    Picked {
        purpose: PickPurpose,
        path: Option<PathBuf>,
    },

    NewDocument(NewSpec),
    OpenFile(PathBuf),
    SaveAs(PathBuf),
    ActivateTab(usize),
    CloseTab {
        index: usize,
        discard: bool,
    },
    CloseAll,
    GoHome,
    Quit,
    ForgetRecent(PathBuf),
    ClearRecent,

    Export(ExportJob),
    RepeatExport,

    ResizeCanvas {
        width: u16,
        height: u16,
        anchor: Anchor,
    },
    SetTiming {
        fps: u16,
        duration: u32,
        looping: bool,
    },

    TogglePaletteEdit,
    AddRamp {
        name: String,
        colors: Vec<Rgb>,
    },
    RemoveRamp(String),
    SetRamp {
        ramp: String,
        name: String,
        colors: Vec<Rgb>,
    },
    GenerateRamp {
        ramp: String,
        base: Rgb,
        steps: u8,
        hue_shift: f64,
        contrast: f64,
        saturation: f64,
    },
    ImportPalette(PathBuf),
    SavePalette(PathBuf),
    ApplyPreset(String),

    SelectElement(Option<NodePath>),
    ReplaceNode {
        path: NodePath,
        text: String,
    },

    SaveAndClose {
        index: usize,
        then: Option<Box<Intent>>,
    },
}
