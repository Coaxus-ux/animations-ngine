mod closing;

use std::path::{Path, PathBuf};

use px_ui::Input;

use super::dialog::{Dialog, DialogKind, PickPurpose};
use super::intent::Intent;
use super::window::{Request, WindowId};
use super::{App, Targets};
use crate::assets::palettes;
use crate::domain::{Document, NewSpec, template};
use crate::ports::FileStore;

pub const KDL_FILTER: &[(&str, &[&str])] = &[("KDL scene", &["kdl"])];

const MAX_UNTITLED: usize = 10_000;

impl App {
    #[expect(
        clippy::unused_self,
        reason = "dialog_for calls every dialog builder as a method"
    )]
    pub(super) fn new_dialog(&self) -> Dialog {
        Dialog::new(DialogKind::New)
    }

    pub(super) fn apply_files(
        &mut self,
        intent: Intent,
        targets: Targets,
        input: &Input,
        store: &mut dyn FileStore,
    ) -> Option<Intent> {
        let source = targets.source;
        let intent = match intent {
            Intent::Save if self.document().is_some_and(Document::is_untitled) => {
                Intent::SaveAs(PathBuf::new())
            }
            other => other,
        };
        match intent {
            Intent::NewDocument(spec) => self.new_document(spec, store),
            Intent::OpenFile(path) if path.as_os_str().is_empty() => {
                self.pick(source, PickPurpose::Open);
            }
            Intent::SaveAs(path) if path.as_os_str().is_empty() => {
                self.pick(source, PickPurpose::SaveAs);
            }
            Intent::OpenFile(path)
            | Intent::Picked {
                purpose: PickPurpose::Open,
                path: Some(path),
            } => self.open_file(&path, store),
            Intent::SaveAs(path)
            | Intent::Picked {
                purpose: PickPurpose::SaveAs,
                path: Some(path),
            } => self.save_as(source, &path, input, store),
            Intent::Picked { path: None, .. } => "Cancelled".clone_into(&mut self.editing.status),
            Intent::CloseTab { index, discard } => self.close_tab(source, index, discard, store),
            Intent::SaveAndClose { index, then } => {
                self.save_and_close(source, index, then, input, store);
            }
            Intent::CloseAll => {
                self.close_all(source, Intent::CloseAll, store);
            }
            Intent::Quit => {
                if self.close_all(source, Intent::Quit, store) {
                    self.quit = true;
                }
            }
            Intent::ForgetRecent(path) => self.recent.remove(&path),
            Intent::ClearRecent => self.recent.clear(),
            other => return Some(other),
        }
        None
    }

    fn new_document(&mut self, spec: NewSpec, store: &mut dyn FileStore) {
        let spec = spec.clamped();
        let Some(preset) = palettes::preset(&spec.preset) else {
            self.editing.status = format!("No palette preset named {:?}", spec.preset);
            return;
        };
        let text = template::new_text(&spec, preset);
        let path = self.untitled_path(&spec.name);
        match Document::new_unsaved(&path, text, store) {
            Ok(document) => {
                self.add_document(document, store);
                self.editing.status = format!(
                    "New {}x{} scene {:?}; it has no file until you save it (Ctrl+S)",
                    spec.width,
                    spec.height,
                    path.display()
                );
            }
            Err(error) => self.editing.status = format!("Could not create the scene: {error}"),
        }
    }

    fn untitled_path(&self, name: &str) -> PathBuf {
        let first = PathBuf::from(format!("{name}.kdl"));
        if self.documents.position(&first).is_none() {
            return first;
        }
        (2..=MAX_UNTITLED)
            .map(|n| PathBuf::from(format!("{name}-{n}.kdl")))
            .find(|path| self.documents.position(path).is_none())
            .unwrap_or(first)
    }

    fn pick(&mut self, window: WindowId, purpose: PickPurpose) {
        let name = match purpose {
            PickPurpose::SaveAs => self
                .document()
                .and_then(|document| document.path().file_name())
                .map(|name| name.to_string_lossy().into_owned()),
            _ => None,
        };
        self.requests.push(Request::PickFile {
            window,
            purpose,
            start: self.last_folder.clone(),
            name,
            filters: KDL_FILTER,
        });
        self.editing.status = match purpose {
            PickPurpose::SaveAs => "Choose where to save the scene".to_owned(),
            _ => "Choose a scene file to open".to_owned(),
        };
    }

    fn open_file(&mut self, path: &Path, store: &mut dyn FileStore) {
        self.leave_active(store);
        match self.open(path, None, store) {
            Ok(()) => {
                self.remember(path);
                self.editing.status = format!("Opened {}", path.display());
            }
            Err(error) => {
                self.editing.status = format!("Could not open {}: {error}", path.display());
            }
        }
    }

    fn save_as(&mut self, source: WindowId, path: &Path, input: &Input, store: &mut dyn FileStore) {
        let Some(document) = self.document_mut() else {
            "Nothing to save: no scene is open".clone_into(&mut self.editing.status);
            return;
        };
        match document.save_to(path, store) {
            Ok(()) => {
                self.remember(path);
                self.editing.status = format!("Saved {}", path.display());
                self.resume_closing(source, input, store);
            }
            Err(error) => {
                self.editing.status = format!("Could not save {}: {error}", path.display());
            }
        }
        self.retitle();
    }

    fn remember(&mut self, path: &Path) {
        self.recent.push(path);
        if let Some(folder) = path
            .parent()
            .filter(|folder| !folder.as_os_str().is_empty())
        {
            self.last_folder = Some(folder.to_path_buf());
        }
    }
}
