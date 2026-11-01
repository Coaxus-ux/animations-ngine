use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use px_engine::{Cel, Geometry, Scene};
use px_format::edit::Source;
use px_format::{LoadOptions, Loaded, Sources, load};

use super::command::Command;
use super::history::History;
use super::layers::{Layer, layers};
use super::template::{NewSpec, new_text};
use crate::assets::palettes;
use crate::ports::FileStore;

/// One open scene file. Its KDL text is the source of truth: every change edits the text
/// and reloads it, and the scene shown is the last text that loaded.
pub struct Document {
    path: PathBuf,
    source: Source,
    loaded: Option<Loaded>,
    /// A scene with in-memory changes not yet written to the text (a stroke in progress).
    preview: Option<Scene>,
    errors: Vec<String>,
    layers: Vec<Layer>,
    history: History,
    revision: u64,
    saved: String,
    stamp: Option<u64>,
    /// Made by `new_unsaved` and never saved: has no file yet.
    untitled: bool,
    /// Files the scene `use`s, as last read.
    others: BTreeMap<PathBuf, String>,
}

impl Document {
    /// # Errors
    /// A readable reason when the file can't be read or isn't KDL.
    pub fn open(path: &Path, store: &mut dyn FileStore) -> Result<Self, String> {
        let text = store.read(path)?;
        let mut document = Self::from_text(path, text, store)?;
        document.stamp = store.stamp(path);
        Ok(document)
    }

    /// # Errors
    /// A readable reason when `text` isn't KDL.
    pub fn from_text(path: &Path, text: String, store: &mut dyn FileStore) -> Result<Self, String> {
        let source = Source::parse(text.clone()).map_err(|error| error.to_string())?;
        let mut document = Self {
            path: path.to_path_buf(),
            source,
            loaded: None,
            preview: None,
            errors: Vec::new(),
            layers: Vec::new(),
            history: History::default(),
            revision: 0,
            saved: text,
            stamp: None,
            untitled: false,
            others: BTreeMap::new(),
        };
        document.reload(store);
        Ok(document)
    }

    /// A document for `text` that has no file yet: dirty until `save_to` gives it one.
    ///
    /// # Errors
    /// A readable reason when `text` isn't KDL.
    pub fn new_unsaved(
        path: &Path,
        text: String,
        store: &mut dyn FileStore,
    ) -> Result<Self, String> {
        let mut document = Self::from_text(path, text, store)?;
        document.saved = String::new();
        document.untitled = true;
        Ok(document)
    }

    /// A new scene file with one empty pixel layer and the default palette preset.
    #[must_use]
    pub fn blank_text(name: &str, width: u16, height: u16) -> String {
        let spec = NewSpec {
            name: name.to_owned(),
            width,
            height,
            ..NewSpec::default()
        };
        new_text(
            &spec,
            palettes::preset(palettes::DEFAULT_PRESET).unwrap_or_default(),
        )
    }

    #[must_use]
    pub const fn is_untitled(&self) -> bool {
        self.untitled
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn text(&self) -> &str {
        self.source.text()
    }

    #[must_use]
    pub const fn source(&self) -> &Source {
        &self.source
    }

    #[must_use]
    pub const fn loaded(&self) -> Option<&Loaded> {
        self.loaded.as_ref()
    }

    /// What to show: the stroke in progress, else the last text that loaded.
    #[must_use]
    pub fn scene(&self) -> Option<&Scene> {
        self.preview
            .as_ref()
            .or_else(|| self.loaded.as_ref().map(|loaded| &loaded.scene))
    }

    #[must_use]
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// Problems with the current text, empty when it loaded.
    #[must_use]
    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    /// Bumped by every change; views redraw when it moves.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.text() != self.saved
    }

    #[must_use]
    pub const fn history(&self) -> &History {
        &self.history
    }

    /// Applies `command` to the text, records it for undo and reloads.
    ///
    /// # Errors
    /// A readable reason when the command can't be applied; nothing changes then.
    pub fn apply(&mut self, command: &Command, store: &mut dyn FileStore) -> Result<(), String> {
        let names = self
            .loaded
            .as_ref()
            .map(|loaded| loaded.paint_names.clone())
            .unwrap_or_default();
        let mut draft = self.source.clone();
        command.apply(&mut draft, &names)?;
        let before = core::mem::replace(&mut self.source, draft);
        self.history.push(command.label(), before.text().to_owned());
        self.reload(store);
        Ok(())
    }

    /// Replaces the whole text (the code view); it is shown once it loads.
    pub fn set_text(&mut self, text: String, store: &mut dyn FileStore) {
        if text == self.text() {
            return;
        }
        match Source::parse(text) {
            Ok(source) => {
                let before = core::mem::replace(&mut self.source, source);
                self.history.push("edit text", before.text().to_owned());
                self.reload(store);
            }
            Err(error) => {
                self.errors = vec![error.to_string()];
                self.revision += 1;
            }
        }
    }

    /// Undoes the last change; returns what was undone.
    pub fn undo(&mut self, store: &mut dyn FileStore) -> Option<String> {
        let (text, label) = self.history.undo(self.text().to_owned())?;
        self.restore(text, store);
        Some(label)
    }

    pub fn redo(&mut self, store: &mut dyn FileStore) -> Option<String> {
        let (text, label) = self.history.redo(self.text().to_owned())?;
        self.restore(text, store);
        Some(label)
    }

    /// Shows `cels` for the pixels of shape `shape` without touching the text yet.
    pub fn preview_cels(&mut self, shape: usize, cels: Vec<Cel>) {
        let Some(loaded) = &self.loaded else { return };
        let mut spec = self
            .preview
            .as_ref()
            .unwrap_or(&loaded.scene)
            .spec()
            .clone();
        if let Some(Geometry::Pixels(art)) =
            spec.shapes.get_mut(shape).map(|shape| &mut shape.geometry)
        {
            art.cels = cels;
        }
        if let Ok(scene) = Scene::new(spec) {
            self.preview = Some(scene);
            self.revision += 1;
        }
    }

    pub fn cancel_preview(&mut self) {
        if self.preview.take().is_some() {
            self.revision += 1;
        }
    }

    /// Writes the text to the file.
    ///
    /// # Errors
    /// A readable reason when it can't be written.
    pub fn save(&mut self, store: &mut dyn FileStore) -> Result<(), String> {
        store.write(&self.path, self.source.text())?;
        self.saved = self.text().to_owned();
        self.stamp = store.stamp(&self.path);
        self.untitled = false;
        self.revision += 1;
        Ok(())
    }

    /// Saves under a new path, which the document keeps.
    ///
    /// # Errors
    /// A readable reason when it can't be written; the path doesn't change then.
    pub fn save_to(&mut self, path: &Path, store: &mut dyn FileStore) -> Result<(), String> {
        let previous = core::mem::replace(&mut self.path, path.to_path_buf());
        self.save(store).inspect_err(|_| self.path = previous)
    }

    /// Reloads the file when it changed on disk and there is nothing unsaved; returns
    /// whether it changed on disk at all.
    pub fn check_disk(&mut self, store: &mut dyn FileStore) -> bool {
        let stamp = store.stamp(&self.path);
        if stamp == self.stamp {
            return false;
        }
        self.stamp = stamp;
        if !self.is_dirty()
            && let Ok(text) = store.read(&self.path)
            && text != self.text()
            && let Ok(source) = Source::parse(text.clone())
        {
            self.source = source;
            self.saved = text;
            self.others.clear();
            self.reload(store);
        }
        true
    }

    fn restore(&mut self, text: String, store: &mut dyn FileStore) {
        match Source::parse(text) {
            Ok(source) => self.source = source,
            Err(error) => self.errors = vec![error.to_string()],
        }
        self.reload(store);
    }

    /// Loads the scene from the text; keeps the last good scene when it fails.
    fn reload(&mut self, store: &mut dyn FileStore) {
        self.preview = None;
        self.revision += 1;
        let mut sources = DocumentSources {
            root: &self.path,
            text: self.source.text(),
            others: &mut self.others,
            store,
        };
        match load(&self.path, &mut sources, LoadOptions::default()) {
            Ok(loaded) => {
                self.errors.clear();
                self.layers = layers(&self.source, &loaded.scene, &loaded.origins);
                self.loaded = Some(loaded);
            }
            Err(error) => {
                self.errors = error
                    .diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.display(&error.files))
                    .collect();
            }
        }
    }
}

/// The document's own text for its path; other files from the store, read once.
struct DocumentSources<'a> {
    root: &'a Path,
    text: &'a str,
    others: &'a mut BTreeMap<PathBuf, String>,
    store: &'a mut dyn FileStore,
}

impl Sources for DocumentSources<'_> {
    fn read(&mut self, path: &Path) -> Result<String, String> {
        if path == self.root {
            return Ok(self.text.to_owned());
        }
        if let Some(text) = self.others.get(path) {
            return Ok(text.clone());
        }
        let text = self.store.read(path)?;
        self.others.insert(path.to_path_buf(), text.clone());
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests fail loudly")]

    use super::*;

    #[derive(Default)]
    struct Memory(BTreeMap<PathBuf, String>);

    impl FileStore for Memory {
        fn read(&mut self, path: &Path) -> Result<String, String> {
            self.0
                .get(path)
                .cloned()
                .ok_or_else(|| "missing".to_owned())
        }
        fn write(&mut self, path: &Path, text: &str) -> Result<(), String> {
            self.0.insert(path.to_path_buf(), text.to_owned());
            Ok(())
        }
        fn stamp(&mut self, path: &Path) -> Option<u64> {
            self.0.get(path).map(|text| text.len() as u64)
        }
    }

    #[test]
    fn an_unsaved_document_is_untitled_and_dirty_until_saved_somewhere() {
        let mut store = Memory::default();
        let text = Document::blank_text("fresh", 8, 8);
        let mut document =
            Document::new_unsaved(Path::new("untitled.kdl"), text.clone(), &mut store).unwrap();
        assert!(document.is_untitled());
        assert!(document.is_dirty());
        assert!(document.errors().is_empty(), "{:?}", document.errors());
        document
            .save_to(Path::new("art/fresh.kdl"), &mut store)
            .unwrap();
        assert!(!document.is_untitled());
        assert!(!document.is_dirty());
        assert_eq!(document.path(), Path::new("art/fresh.kdl"));
        assert_eq!(store.0[Path::new("art/fresh.kdl")], text);
    }

    #[test]
    fn a_blank_scene_uses_the_default_preset_palette() {
        let text = Document::blank_text("fresh", 8, 8);
        assert!(text.starts_with(&format!("palette {:?}", palettes::DEFAULT_PRESET)));
        assert!(text.contains("scene \"fresh\" palette=\"basic-16\" width=8 height=8"));
    }
}
