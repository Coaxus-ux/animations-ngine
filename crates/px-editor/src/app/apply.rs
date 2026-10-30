use kdl::KdlValue;
use px_ui::Input;

use super::intent::Intent;
use super::selection_ops::SelectionOp;
use super::window::{CodeMode, Picker, WindowKind};
use super::{App, WindowId};
use crate::domain::{Command, Selection};
use crate::ports::FileStore;

impl App {
    /// Applies `intent` as if window `source` asked for it: for scripts and tests.
    pub fn dispatch(&mut self, intent: Intent, source: WindowId, store: &mut dyn FileStore) {
        self.apply(intent, source, &Input::default(), store);
    }

    /// Applies one intent from window `source`. Canvas intents act on that window; panel
    /// intents on the canvas window last used.
    pub(super) fn apply(
        &mut self,
        intent: Intent,
        source: WindowId,
        input: &Input,
        store: &mut dyn FileStore,
    ) {
        let canvas = match self.window(source) {
            Some(window) if window.kind.has_canvas() => source,
            _ => self.focused,
        };
        let Some(intent) = self.apply_paints(intent, source, store) else {
            return;
        };
        let Some(intent) = self.apply_layers(intent, source, canvas, store) else {
            return;
        };
        let Some(intent) = self.apply_canvas(intent, canvas, input, store) else {
            return;
        };
        self.apply_workspace(&intent, source, canvas, store);
    }

    /// Tools, paints and palette colors; returns the intent when it is about something else.
    fn apply_paints(
        &mut self,
        intent: Intent,
        source: WindowId,
        store: &mut dyn FileStore,
    ) -> Option<Intent> {
        match intent {
            Intent::Tool(tool) => {
                self.editing.tool = tool;
                tool.tip().clone_into(&mut self.editing.status);
            }
            Intent::Filled(filled) => self.editing.filled = filled,
            Intent::PrimaryPaint(paint) => self.editing.primary = paint,
            Intent::SecondaryPaint(paint) => self.editing.secondary = paint,
            Intent::SwapPaints => {
                core::mem::swap(&mut self.editing.primary, &mut self.editing.secondary);
            }
            Intent::Swatch(paint) => self.swatch(paint, store),
            Intent::OpenPicker(paint) => {
                let color = self
                    .document
                    .as_ref()
                    .and_then(crate::domain::Document::scene)
                    .and_then(|scene| scene.palette().resolve(paint).ok())
                    .unwrap_or_default();
                self.window_mut(source, |window| {
                    window.picker = Some(Picker { paint, color });
                });
            }
            Intent::ClosePicker => self.window_mut(source, |window| window.picker = None),
            Intent::SetColor { paint, color } => self.set_color(paint, color, store),
            Intent::GlobalFill(global) => self.editing.global_fill = global,
            Intent::RecolorPicked => {
                if let Some(paint) = self.editing.primary {
                    self.recolor_picked(paint, store);
                }
            }
            Intent::EditPrimaryColor => {
                if let Some(paint) = self.editing.primary {
                    return Some(Intent::OpenPicker(paint));
                }
            }
            other => return Some(other),
        }
        None
    }

    /// The layer list: selecting, showing, locking, adding, moving, renaming.
    fn apply_layers(
        &mut self,
        intent: Intent,
        source: WindowId,
        canvas: WindowId,
        store: &mut dyn FileStore,
    ) -> Option<Intent> {
        match intent {
            Intent::SelectLayer(layer) => self.view_mut(canvas, |view| view.layer = Some(layer)),
            Intent::ToggleHidden(name) => self.view_mut(canvas, |view| {
                if !view.hidden.remove(&name) {
                    view.hidden.insert(name);
                }
            }),
            Intent::ToggleLocked(name) => {
                if !self.editing.locked.remove(&name) {
                    self.editing.locked.insert(name);
                }
            }
            Intent::AddLayer => self.add_layer(canvas, store),
            Intent::DeleteLayer(layer) => {
                if let Some(path) = self.layer(layer).map(|layer| layer.path.clone()) {
                    self.run(&Command::Remove { path }, store);
                }
            }
            Intent::MoveLayer { from, to } => self.move_layer(from, to, store),
            Intent::StartRename(layer) => {
                self.window_mut(source, |window| window.renaming = Some(layer));
            }
            Intent::RenameLayer { layer, name } => {
                self.window_mut(source, |window| window.renaming = None);
                self.rename_layer(layer, name, store);
            }
            Intent::Opacity { layer, value } => {
                if let Some(art) = self.layer(layer).and_then(|layer| layer.art.clone()) {
                    let value = KdlValue::Float((value * 100.0).round() / 100.0);
                    let key = "opacity".to_owned();
                    self.run(
                        &Command::SetProperty {
                            path: art.path,
                            key,
                            value,
                        },
                        store,
                    );
                }
            }
            other => return Some(other),
        }
        None
    }

    /// Frames, the canvas view, strokes and the document's text and history.
    fn apply_canvas(
        &mut self,
        intent: Intent,
        canvas: WindowId,
        input: &Input,
        store: &mut dyn FileStore,
    ) -> Option<Intent> {
        match intent {
            Intent::Frame(frame) => {
                let duration = self.duration();
                self.view_mut(canvas, |view| view.frame = frame % duration);
            }
            Intent::StepFrame(step) => {
                let duration = i64::from(self.duration());
                self.view_mut(canvas, |view| {
                    let frame = (i64::from(view.frame) + i64::from(step)).rem_euclid(duration);
                    view.frame = u32::try_from(frame).unwrap_or(0);
                });
            }
            Intent::TogglePlay => self.editing.playing = !self.editing.playing,
            Intent::ToggleOnion => self.view_mut(canvas, |view| view.onion = !view.onion),
            Intent::ToggleGrid => self.view_mut(canvas, |view| view.grid = !view.grid),
            Intent::NewCel => self.new_cel(canvas, store),
            Intent::DeleteCel => self.delete_cel(canvas, store),
            Intent::AddFrame => self.resize_timeline(1, store),
            Intent::DeleteFrame => self.resize_timeline(-1, store),
            Intent::Zoom { steps, at } => self.zoom(canvas, steps, at),
            Intent::ZoomFit => self.view_mut(canvas, |view| {
                view.zoom = 0;
                view.pan = None;
            }),
            Intent::Pan { dx, dy } => self.pan(canvas, dx, dy),
            Intent::CanvasPress { at, button } => {
                self.begin_stroke(canvas, at, button, input, store);
            }
            Intent::CanvasDrag { at } => self.drag_stroke(at, input, store),
            Intent::CanvasRelease { at } => self.end_stroke(at, input, store),
            Intent::SetProperty { path, key, value } => {
                self.run(&Command::SetProperty { path, key, value }, store);
            }
            Intent::SetText(text) => {
                if let Some(document) = &mut self.document {
                    document.set_text(text, store);
                }
            }
            Intent::Undo => self.history(true, store),
            Intent::Redo => self.history(false, store),
            Intent::Save => self.save(store),
            other => return Some(other),
        }
        None
    }

    /// Selections, windows and the theme.
    fn apply_workspace(
        &mut self,
        intent: &Intent,
        source: WindowId,
        canvas: WindowId,
        store: &mut dyn FileStore,
    ) {
        match *intent {
            Intent::Copy => self.selection_op(canvas, SelectionOp::Copy, store),
            Intent::Cut => self.selection_op(canvas, SelectionOp::Cut, store),
            Intent::Paste => self.selection_op(canvas, SelectionOp::Paste, store),
            Intent::Flip { horizontal } => {
                self.selection_op(canvas, SelectionOp::Flip { horizontal }, store);
            }
            Intent::DeleteSelection => self.selection_op(canvas, SelectionOp::Delete, store),
            Intent::SelectAll => {
                let (width, height) = self.canvas_size();
                self.editing.selection =
                    Selection::rectangle(width, height, (0, 0), (width - 1, height - 1));
            }
            Intent::Deselect => {
                let (width, height) = self.canvas_size();
                self.editing.selection = Selection::none(width, height);
            }
            Intent::CycleCode => {
                if let Some(main) = self.main_mut() {
                    main.view.code = match main.view.code {
                        CodeMode::Hidden => CodeMode::Split,
                        CodeMode::Split => CodeMode::Only,
                        CodeMode::Only => CodeMode::Hidden,
                    };
                }
            }
            Intent::NewWindow => self.focused = self.open_window(WindowKind::Canvas),
            Intent::PopOut(panel) => self.pop_out(panel),
            Intent::Dock(panel) => self.dock(panel),
            Intent::SetTheme(theme) => {
                if theme != self.theme {
                    self.theme = theme;
                    self.reload_assets();
                }
            }
            Intent::SetScale(scale) => self.set_scale(scale),
            Intent::SetCode(mode) => {
                if let Some(main) = self.main_mut() {
                    main.view.code = mode;
                }
            }
            Intent::ToggleHelp => self.window_mut(source, |window| window.help = !window.help),
            _ => {}
        }
    }

    pub(super) fn window_mut(&mut self, id: WindowId, change: impl FnOnce(&mut super::Window)) {
        if let Some(window) = self.windows.iter_mut().find(|window| window.id == id) {
            change(window);
        }
    }

    pub(super) fn view_mut(
        &mut self,
        window: WindowId,
        change: impl FnOnce(&mut super::ViewState),
    ) {
        if let Some(window) = self.windows.iter_mut().find(|other| other.id == window) {
            change(&mut window.view);
        }
    }
}
