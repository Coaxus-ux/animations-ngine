use kdl::KdlValue;
use px_ui::Input;

use super::intent::Intent;
use super::selection_ops::SelectionOp;
use super::window::{Picker, WindowKind};
use super::{App, WindowId};
use crate::app::Status;
use crate::domain::{Command, Selection};
use crate::ports::FileStore;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Targets {
    pub source: WindowId,
    pub canvas: WindowId,
}

type Handler = fn(&mut App, Intent, Targets, &Input, &mut dyn FileStore) -> Option<Intent>;

const HANDLERS: &[Handler] = &[
    App::apply_dialogs,
    App::apply_files,
    App::apply_tabs,
    App::apply_export,
    App::apply_palette,
    App::apply_nodes,
    App::apply_paints,
    App::apply_layers,
    App::apply_canvas,
    App::apply_workspace,
];

impl App {
    pub fn dispatch(&mut self, intent: Intent, source: WindowId, store: &mut dyn FileStore) {
        self.apply(intent, source, &Input::default(), store);
    }

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
        let targets = Targets { source, canvas };
        let mut pending = Some(intent);
        for handler in HANDLERS {
            let Some(intent) = pending.take() else { return };
            pending = handler(self, intent, targets, input, store);
        }
    }

    fn apply_paints(
        &mut self,
        intent: Intent,
        targets: Targets,
        _input: &Input,
        store: &mut dyn FileStore,
    ) -> Option<Intent> {
        let source = targets.source;
        match intent {
            Intent::Tool(tool) => {
                self.editing.tool = tool;
                self.editing.status = Status::info(tool.tip());
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
                    .document()
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

    fn apply_layers(
        &mut self,
        intent: Intent,
        targets: Targets,
        _input: &Input,
        store: &mut dyn FileStore,
    ) -> Option<Intent> {
        let Targets { source, canvas } = targets;
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
            Intent::CancelRename => {
                self.window_mut(source, |window| window.renaming = None);
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

    fn apply_canvas(
        &mut self,
        intent: Intent,
        targets: Targets,
        input: &Input,
        store: &mut dyn FileStore,
    ) -> Option<Intent> {
        let canvas = targets.canvas;
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
                if let Some(document) = self.document_mut() {
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

    fn apply_workspace(
        &mut self,
        intent: Intent,
        targets: Targets,
        _input: &Input,
        store: &mut dyn FileStore,
    ) -> Option<Intent> {
        let Targets { source, canvas } = targets;
        match intent {
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
                if let Some(mode) = self.main_mut().map(|main| main.view.code.next()) {
                    self.set_code(mode);
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
            Intent::SetCode(mode) => self.set_code(mode),
            Intent::TogglePaletteSide => {
                if let Some(main) = self.main_mut() {
                    main.panels.palette_right = !main.panels.palette_right;
                }
            }
            Intent::CopyText(text) => self.requests.push(super::Request::Clipboard(text)),
            Intent::ToggleHelp => self.window_mut(source, |window| window.help = !window.help),
            other => return Some(other),
        }
        None
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
