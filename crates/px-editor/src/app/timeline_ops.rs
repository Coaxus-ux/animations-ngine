use kdl::KdlValue;

use super::{App, WindowId};
use crate::domain::{Command, scene_path};
use crate::ports::FileStore;

impl App {
    /// Starts a new cel at the view's frame, a copy of the one showing there.
    pub(super) fn new_cel(&mut self, window: WindowId, store: &mut dyn FileStore) {
        let Some(frame) = self.window(window).map(|window| window.view.frame) else {
            return;
        };
        match self.paint_target(window) {
            Ok(target) => {
                let target = target.starting_at(frame);
                if target.has_cel() {
                    self.editing.status = format!("A cel already starts at frame {}", frame + 1);
                    return;
                }
                let command = target.command(&target.raster);
                self.run(&command, store);
            }
            Err(reason) => self.editing.status = reason,
        }
    }

    /// Removes the cel starting at the view's frame, so the one before shows on.
    pub(super) fn delete_cel(&mut self, window: WindowId, store: &mut dyn FileStore) {
        let Some(frame) = self.window(window).map(|window| window.view.frame) else {
            return;
        };
        match self.paint_target(window) {
            Ok(target) => {
                let target = target.starting_at(frame);
                if !target.has_cel() {
                    self.editing.status = format!("No cel starts at frame {}", frame + 1);
                    return;
                }
                let command = target.without_cel();
                self.run(&command, store);
            }
            Err(reason) => self.editing.status = reason,
        }
    }

    /// Makes the scene `delta` frames longer (or shorter), at least one frame.
    pub(super) fn resize_timeline(&mut self, delta: i64, store: &mut dyn FileStore) {
        let Some(scene) = self
            .document
            .as_ref()
            .and_then(crate::domain::Document::scene)
        else {
            return;
        };
        let frames = (i64::from(scene.timing().duration.get()) + delta).max(1);
        let command = Command::SetProperty {
            path: scene_path(scene),
            key: "duration".to_owned(),
            value: KdlValue::String(format!("{frames}f")),
        };
        self.run(&command, store);
    }

    pub(super) fn run(&mut self, command: &Command, store: &mut dyn FileStore) {
        if let Some(document) = &mut self.document {
            match document.apply(command, store) {
                Ok(()) => self.editing.status = format!("Done: {}", command.label()),
                Err(error) => self.editing.status = error,
            }
        }
    }
}
