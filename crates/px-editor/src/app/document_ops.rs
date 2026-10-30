use kdl::KdlValue;
use px_engine::Geometry;

use super::{App, WindowId};
use crate::domain::{Command, scene_path};
use crate::ports::FileStore;

const MAX_LAYERS: usize = 10_000;

impl App {
    /// A swatch click: recolors the picked object with the pick tool, else picks the paint.
    pub(super) fn swatch(&mut self, paint: px_core::Paint, store: &mut dyn FileStore) {
        if self.editing.tool == crate::tools::ToolKind::Pick && self.editing.picked.is_some() {
            self.recolor_picked(paint, store);
        } else {
            self.editing.primary = Some(paint);
        }
    }

    /// Rewrites the picked object's `paint` (shapes; pixel layers keep their own paints).
    pub(super) fn recolor_picked(&mut self, paint: px_core::Paint, store: &mut dyn FileStore) {
        let target = self.editing.picked.and_then(|shape| {
            let document = self.document.as_ref()?;
            let loaded = document.loaded()?;
            let origin = loaded
                .origins
                .get(shape)
                .filter(|origin| origin.file == 0)?;
            let pixels = matches!(
                loaded.scene.shapes().get(shape)?.geometry,
                Geometry::Pixels(_)
            );
            let name = loaded.paint_names.paint(paint)?;
            (!pixels).then(|| (document.source().path_at(origin.offset), name))
        });
        if let Some((Some(path), name)) = target {
            let command = Command::SetProperty {
                path,
                key: "paint".to_owned(),
                value: KdlValue::String(name),
            };
            self.run(&command, store);
        } else {
            "Pick a shape first (pick tool, O); pixel layers keep their own colors"
                .clone_into(&mut self.editing.status);
        }
    }

    pub(super) fn set_color(
        &mut self,
        paint: px_core::Paint,
        color: px_core::Rgb,
        store: &mut dyn FileStore,
    ) {
        let Some(document) = &self.document else {
            return;
        };
        let Some(loaded) = document.loaded() else {
            return;
        };
        let Some(ramp) = loaded.paint_names.ramp(paint.ramp) else {
            return;
        };
        let palette = document
            .source()
            .node(&scene_path(&loaded.scene))
            .and_then(|scene| scene.entry("palette"))
            .and_then(|entry| entry.value().as_string())
            .map(str::to_owned);
        let Some(palette) = palette else { return };
        let path = px_format::edit::NodePath::root()
            .child("palette", Some(&palette), 0)
            .child("ramp", Some(ramp), 0);
        if document.source().node(&path).is_none() {
            self.editing.status = format!("Palette \"{palette}\" lives in another file");
            return;
        }
        let tone = usize::from(paint.tone.index());
        self.run(
            &Command::SetColor {
                ramp: path,
                tone,
                color,
            },
            store,
        );
    }

    pub(super) fn add_layer(&mut self, window: WindowId, store: &mut dyn FileStore) {
        let Some(document) = &self.document else {
            return;
        };
        let Some(scene) = document.scene() else {
            return;
        };
        let taken = |name: &str| document.layers().iter().any(|layer| layer.name == name);
        let name = (1..=MAX_LAYERS)
            .map(|n| format!("layer-{n}"))
            .find(|name| !taken(name))
            .unwrap_or_default();
        let active = self.window(window).and_then(|window| window.view.layer);
        let child = active
            .and_then(|index| document.layers().get(index))
            .map_or(usize::MAX, |layer| layer.child + 1);
        let command = Command::AddLayer {
            scene: scene_path(scene),
            child,
            name: name.clone(),
        };
        self.run(&command, store);
        let index = self.document.as_ref().and_then(|document| {
            document
                .layers()
                .iter()
                .position(|layer| layer.name == name)
        });
        self.view_mut(window, |view| view.layer = index);
    }

    pub(super) fn move_layer(&mut self, from: usize, to: usize, store: &mut dyn FileStore) {
        let Some(document) = &self.document else {
            return;
        };
        let Some(scene) = document.scene() else {
            return;
        };
        let layers = document.layers();
        let (Some(moving), Some(target)) = (
            layers.get(from),
            layers.get(to.min(layers.len().saturating_sub(1))),
        ) else {
            return;
        };
        // Counted after the moving layer is taken out, the target's index puts it on the
        // far side of the target in either direction.
        let command = Command::MoveChild {
            scene: scene_path(scene),
            from: moving.child,
            to: target.child,
        };
        self.run(&command, store);
        for window in &mut self.windows {
            if window.view.layer == Some(from) {
                window.view.layer = Some(
                    to.min(
                        self.document
                            .as_ref()
                            .map_or(0, |doc| doc.layers().len().saturating_sub(1)),
                    ),
                );
            }
        }
    }

    pub(super) fn rename_layer(&mut self, layer: usize, name: String, store: &mut dyn FileStore) {
        if let Some(layer) = self.layer(layer).cloned()
            && !name.is_empty()
            && name != layer.name
        {
            let art = layer.art.as_ref().map(|art| art.path.clone());
            self.run(
                &Command::Rename {
                    path: layer.path,
                    art,
                    name,
                },
                store,
            );
        }
    }

    pub(super) fn history(&mut self, undo: bool, store: &mut dyn FileStore) {
        let Some(document) = &mut self.document else {
            return;
        };
        let label = if undo {
            document.undo(store)
        } else {
            document.redo(store)
        };
        self.editing.status = match (label, undo) {
            (Some(label), true) => format!("Undid {label}"),
            (Some(label), false) => format!("Redid {label}"),
            (None, true) => "Nothing to undo".to_owned(),
            (None, false) => "Nothing to redo".to_owned(),
        };
    }

    pub(super) fn save(&mut self, store: &mut dyn FileStore) {
        let Some(document) = &mut self.document else {
            return;
        };
        self.editing.status = match document.save(store) {
            Ok(()) => format!("Saved {}", document.path().display()),
            Err(error) => format!("Could not save: {error}"),
        };
        self.retitle();
    }
}
