use kdl::KdlValue;
use px_engine::{Cel, Geometry, Scene};
use px_format::edit::NodePath;
use px_ui::Input;

use super::dialog::{Dialog, DialogKind, keys};
use super::intent::Intent;
use super::window::WindowKind;
use super::{App, Targets, WindowId};
use crate::domain::{Anchor, Command, Document, Selection, resize, scene_path};
use crate::ports::FileStore;

impl App {
    /// Canvas Size with the scene's current size filled in.
    pub(super) fn canvas_size_dialog(&self) -> Dialog {
        let mut dialog = Dialog::new(DialogKind::CanvasSize);
        let (width, height) = self.canvas_size();
        dialog.form.number(keys::WIDTH, i64::from(width));
        dialog.form.number(keys::HEIGHT, i64::from(height));
        dialog.form.choice(keys::ANCHOR, Anchor::default_index());
        dialog
    }

    /// Properties with the scene's name, timing and background filled in.
    pub(super) fn properties_dialog(&self) -> Dialog {
        let mut dialog = Dialog::new(DialogKind::Properties);
        let Some(document) = self.document() else {
            return dialog;
        };
        let Some(scene) = document.scene() else {
            return dialog;
        };
        let timing = scene.timing();
        let background = document
            .source()
            .node(&scene_path(scene))
            .and_then(|node| node.get(keys::BACKGROUND))
            .and_then(KdlValue::as_string)
            .unwrap_or_default();
        dialog.form.text(keys::NAME, scene.name());
        dialog.form.number(keys::FPS, i64::from(timing.fps.get()));
        dialog
            .form
            .number(keys::DURATION, i64::from(timing.duration.get()));
        dialog
            .form
            .flag(keys::LOOP, timing.looping == px_engine::Looping::Loop);
        dialog.form.text(keys::BACKGROUND, background);
        dialog
    }

    pub(super) fn apply_nodes(
        &mut self,
        intent: Intent,
        targets: Targets,
        _input: &Input,
        store: &mut dyn FileStore,
    ) -> Option<Intent> {
        match intent {
            Intent::SelectElement(path) => self.select_element(targets.source, path.as_ref()),
            Intent::ReplaceNode { path, text } => {
                self.replace_node(targets.source, &path, text, store);
            }
            Intent::ResizeCanvas {
                width,
                height,
                anchor,
            } => self.resize_canvas((width, height), anchor, store),
            Intent::SetTiming {
                fps,
                duration,
                looping,
            } => self.set_timing(fps, duration, looping, store),
            other => return Some(other),
        }
        None
    }

    /// The selection lives in the window that made it and in the main window, so a torn-off
    /// Elements panel steers the main window's code view and inspector.
    fn select_element(&mut self, source: WindowId, path: Option<&NodePath>) {
        self.editing.picked = path.and_then(|path| self.shape_at(path));
        for window in &mut self.windows {
            if window.id == source || window.kind == WindowKind::Main {
                window.selected = path.cloned();
                window.code.jumped = None;
            }
        }
    }

    /// The shape built from the node at `path`, if it is one.
    fn shape_at(&self, path: &NodePath) -> Option<usize> {
        let document = self.document()?;
        let (start, _) = document.source().span(path)?;
        document
            .loaded()?
            .origins
            .iter()
            .position(|origin| origin.file == 0 && origin.offset == start)
    }

    /// The node's own path changes when its id does, so the selection is found again by
    /// where the node starts; a refused edit leaves the text, and so the path, as it was.
    fn replace_node(
        &mut self,
        source: WindowId,
        path: &NodePath,
        text: String,
        store: &mut dyn FileStore,
    ) {
        let start = self
            .document()
            .and_then(|document| document.source().span(path))
            .map(|(start, _)| start);
        self.run(
            &Command::ReplaceNode {
                path: path.clone(),
                text,
            },
            store,
        );
        let selected = start.and_then(|start| self.document()?.source().path_at(start));
        self.select_element(source, selected.as_ref());
    }

    fn resize_canvas(&mut self, size: (u16, u16), anchor: Anchor, store: &mut dyn FileStore) {
        let Some(document) = self.document() else {
            return;
        };
        let Some(scene) = document.scene() else {
            return;
        };
        let old = (scene.size().width(), scene.size().height());
        let (dx, dy) = resize::offset(anchor, old, size);
        let command = Command::ResizeCanvas {
            scene: scene_path(scene),
            width: size.0,
            height: size.1,
            arts: shifted_arts(document, scene, (dx, dy), size),
        };
        if self.ran(&command, store) {
            self.editing.selection = Selection::none(i32::from(size.0), i32::from(size.1));
            self.editing.status = format!(
                "Canvas is now {}×{} ({} anchored); undo brings the old size back",
                size.0,
                size.1,
                anchor_name(anchor)
            );
        }
    }

    /// One `SetProperty` per changed value: each is its own undo step, which the status says.
    fn set_timing(&mut self, fps: u16, duration: u32, looping: bool, store: &mut dyn FileStore) {
        let Some(scene) = self.document().and_then(Document::scene) else {
            return;
        };
        let timing = scene.timing();
        let path = scene_path(scene);
        let changes: Vec<(&str, KdlValue)> = [
            (timing.fps.get() != fps).then(|| (keys::FPS, KdlValue::Integer(fps.into()))),
            (timing.duration.get() != duration)
                .then(|| (keys::DURATION, KdlValue::String(format!("{duration}f")))),
            ((timing.looping == px_engine::Looping::Loop) != looping)
                .then_some((keys::LOOP, KdlValue::Bool(looping))),
        ]
        .into_iter()
        .flatten()
        .collect();
        if changes.is_empty() {
            "Timing is already like that".clone_into(&mut self.editing.status);
            return;
        }
        let steps = changes.len();
        let all_ran = changes.into_iter().all(|(key, value)| {
            let command = Command::SetProperty {
                path: path.clone(),
                key: key.to_owned(),
                value,
            };
            self.ran(&command, store)
        });
        if all_ran {
            let looping = if looping { ", looping" } else { ", once" };
            self.editing.status = format!(
                "Timing is now {fps} fps, {duration} frames{looping} ({steps} undo step{})",
                if steps == 1 { "" } else { "s" }
            );
        }
    }
}

impl App {
    /// Runs the command and tells whether the document took it (its revision moved).
    fn ran(&mut self, command: &Command, store: &mut dyn FileStore) -> bool {
        let before = self.document().map(Document::revision);
        self.run(command, store);
        self.document().map(Document::revision) != before
    }
}

/// Every pixel layer's cels moved by `(dx, dy)` and cropped to the new canvas. Cels sit in
/// their layer's coordinates while the canvas crops in its own, so a layer's origin is added
/// for the crop and taken off again. A layer whose position is an expression has no fixed
/// place on the canvas, so its cels are left as they are.
fn shifted_arts(
    document: &Document,
    scene: &Scene,
    (dx, dy): (i32, i32),
    size: (u16, u16),
) -> Vec<(NodePath, Vec<Cel>)> {
    document
        .layers()
        .iter()
        .filter_map(|layer| {
            let art = layer.art.as_ref()?;
            let Geometry::Pixels(pixels) = &scene.shapes().get(art.shape)?.geometry else {
                return None;
            };
            let (ox, oy) = art.origin?;
            let mut cels = resize::shifted_cels(&pixels.cels, dx + ox, dy + oy, size);
            for cel in &mut cels {
                cel.x -= ox;
                cel.y -= oy;
            }
            Some((art.path.clone(), cels))
        })
        .collect()
}

fn anchor_name(anchor: Anchor) -> &'static str {
    match anchor {
        Anchor::TopLeft => "top left",
        Anchor::Top => "top",
        Anchor::TopRight => "top right",
        Anchor::Left => "left",
        Anchor::Center => "center",
        Anchor::Right => "right",
        Anchor::BottomLeft => "bottom left",
        Anchor::Bottom => "bottom",
        Anchor::BottomRight => "bottom right",
    }
}
