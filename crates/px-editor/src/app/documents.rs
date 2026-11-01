use std::path::Path;

use super::editing::Editing;
use super::window::ViewState;
use crate::domain::Document;

/// Tells one open document from another for caches, whatever its path or tab position.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DocumentId(pub u32);

/// One open document with the state kept for it while another tab is active: the active
/// document's `Editing` and main view live in `App` and are swapped in here on tab change.
pub struct Slot {
    pub id: DocumentId,
    pub document: Document,
    pub editing: Editing,
    pub view: ViewState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabInfo {
    pub title: String,
    pub dirty: bool,
}

#[derive(Default)]
pub struct Documents {
    slots: Vec<Slot>,
    active: Option<usize>,
    next: u32,
}

impl Documents {
    pub fn open(&mut self, document: Document) -> DocumentId {
        let id = DocumentId(self.next);
        self.next += 1;
        self.slots.push(Slot {
            id,
            document,
            editing: Editing::default(),
            view: ViewState::default(),
        });
        self.active = Some(self.slots.len() - 1);
        id
    }

    /// Removes tab `index`; the active tab moves to the nearest remaining one.
    pub fn close(&mut self, index: usize) -> Option<Slot> {
        if index >= self.slots.len() {
            return None;
        }
        let slot = self.slots.remove(index);
        self.active = match self.active {
            _ if self.slots.is_empty() => None,
            Some(active) if active > index => Some(active - 1),
            Some(active) if active == index => Some(index.min(self.slots.len() - 1)),
            other => other,
        };
        Some(slot)
    }

    pub fn activate(&mut self, index: usize) -> bool {
        if index < self.slots.len() {
            self.active = Some(index);
            true
        } else {
            false
        }
    }

    /// Shows the Home tab while every document stays open.
    pub fn deactivate(&mut self) {
        self.active = None;
    }

    #[must_use]
    pub const fn active_index(&self) -> Option<usize> {
        self.active
    }

    #[must_use]
    pub fn active(&self) -> Option<&Slot> {
        self.slots.get(self.active?)
    }

    pub fn active_mut(&mut self) -> Option<&mut Slot> {
        self.slots.get_mut(self.active?)
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Slot> {
        self.slots.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Slot> {
        self.slots.get_mut(index)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Slot> {
        self.slots.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Slot> {
        self.slots.iter_mut()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    #[must_use]
    pub fn position(&self, path: &Path) -> Option<usize> {
        self.slots
            .iter()
            .position(|slot| slot.document.path() == path)
    }

    #[must_use]
    pub fn tabs(&self) -> Vec<TabInfo> {
        self.slots
            .iter()
            .map(|slot| TabInfo {
                title: slot.document.path().file_name().map_or_else(
                    || "untitled".to_owned(),
                    |name| name.to_string_lossy().into_owned(),
                ),
                dirty: slot.document.is_dirty(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::ports::NoFiles;

    fn document(name: &str) -> Document {
        let text = Document::blank_text(name, 4, 4);
        Document::from_text(&PathBuf::from(format!("{name}.kdl")), text, &mut NoFiles).unwrap()
    }

    #[test]
    fn closing_the_active_tab_activates_its_neighbour_and_closing_the_last_leaves_none() {
        let mut documents = Documents::default();
        documents.open(document("a"));
        documents.open(document("b"));
        documents.open(document("c"));
        assert!(documents.activate(1));
        documents.close(1);
        assert_eq!(documents.active().map(|slot| slot.id), Some(DocumentId(2)));
        documents.close(1);
        documents.close(0);
        assert!(documents.active().is_none() && documents.is_empty());
    }

    #[test]
    fn closing_a_tab_before_the_active_one_keeps_the_same_document_active() {
        let mut documents = Documents::default();
        documents.open(document("a"));
        documents.open(document("b"));
        documents.close(0);
        assert_eq!(documents.active_index(), Some(0));
        assert_eq!(documents.active().map(|slot| slot.id), Some(DocumentId(1)));
    }
}
