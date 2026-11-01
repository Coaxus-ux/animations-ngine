use std::collections::BTreeMap;

use px_ui::Input;

use super::intent::Intent;
use super::{App, Targets};
use crate::ports::FileStore;

/// Field names shared by the app side that fills a form and the view that reads it.
pub mod keys {
    pub const NAME: &str = "name";
    pub const WIDTH: &str = "width";
    pub const HEIGHT: &str = "height";
    pub const FPS: &str = "fps";
    pub const FRAMES: &str = "frames";
    pub const DURATION: &str = "duration";
    pub const LOOP: &str = "loop";
    pub const BACKGROUND: &str = "background";
    pub const PRESET: &str = "preset";
    pub const ANCHOR: &str = "anchor";
    pub const KIND: &str = "kind";
    pub const LAYOUT: &str = "layout";
    pub const COUNT: &str = "count";
    pub const BORDER: &str = "border";
    pub const SHAPE: &str = "shape";
    pub const JSON: &str = "json";
    pub const SCALE: &str = "scale";
    pub const TRANSPARENT: &str = "transparent";
    pub const OUT: &str = "out";
    /// The tab a Confirm dialog asks about.
    pub const TAB: &str = "tab";
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogKind {
    New,
    Export,
    /// The Export dialog opened on the sprite-sheet kind.
    ExportSheet,
    CanvasSize,
    Properties,
    /// "Save changes?" before an action that would lose them.
    Confirm,
}

/// What a native file dialog is for, so its answer goes to the right place.
impl DialogKind {
    #[must_use]
    pub const fn needs_document(self) -> bool {
        !matches!(self, Self::New | Self::Confirm)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickPurpose {
    Open,
    SaveAs,
    ExportOut,
    ImportPalette,
    SavePalette,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Form {
    pub texts: BTreeMap<String, String>,
    pub numbers: BTreeMap<String, i64>,
    pub choices: BTreeMap<String, usize>,
    pub flags: BTreeMap<String, bool>,
    pub message: String,
}

impl Form {
    pub fn number(&mut self, key: &str, default: i64) -> &mut i64 {
        self.numbers.entry(key.to_owned()).or_insert(default)
    }

    pub fn text(&mut self, key: &str, default: &str) -> &mut String {
        self.texts
            .entry(key.to_owned())
            .or_insert_with(|| default.to_owned())
    }

    pub fn choice(&mut self, key: &str, default: usize) -> &mut usize {
        self.choices.entry(key.to_owned()).or_insert(default)
    }

    pub fn flag(&mut self, key: &str, default: bool) -> &mut bool {
        self.flags.entry(key.to_owned()).or_insert(default)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dialog {
    pub kind: DialogKind,
    pub form: Form,
    /// What a Confirm dialog does once the changes are saved or discarded.
    pub then: Option<Box<Intent>>,
}

impl Dialog {
    #[must_use]
    pub fn new(kind: DialogKind) -> Self {
        Self {
            kind,
            form: Form::default(),
            then: None,
        }
    }
}

impl App {
    pub(super) fn apply_dialogs(
        &mut self,
        intent: Intent,
        targets: Targets,
        _input: &Input,
        _store: &mut dyn FileStore,
    ) -> Option<Intent> {
        match intent {
            Intent::ShowDialog(kind) => {
                if kind.needs_document() && self.document().is_none() {
                    "Open a scene first".clone_into(&mut self.editing.status);
                    return None;
                }
                let dialog = self.dialog_for(kind);
                self.window_mut(targets.source, |window| {
                    window.dialog = Some(dialog);
                    window.ui.focus = None;
                });
            }
            Intent::CloseDialog => self.window_mut(targets.source, |window| window.dialog = None),
            other => return Some(other),
        }
        None
    }

    fn dialog_for(&self, kind: DialogKind) -> Dialog {
        match kind {
            DialogKind::New => self.new_dialog(),
            DialogKind::Export => self.export_dialog(false),
            DialogKind::ExportSheet => self.export_dialog(true),
            DialogKind::CanvasSize => self.canvas_size_dialog(),
            DialogKind::Properties => self.properties_dialog(),
            DialogKind::Confirm => Dialog::new(kind),
        }
    }
}
