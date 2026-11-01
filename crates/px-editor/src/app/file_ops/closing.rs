//! Closing tabs that may have unsaved changes: the Confirm dialog, pinned to the tab it
//! asks about, and its Save that waits for a native pick when the tab has no file yet.

use px_ui::Input;

use super::super::App;
use super::super::dialog::{Dialog, DialogKind, PickPurpose, keys};
use super::super::intent::Intent;
use super::super::window::WindowId;
use crate::domain::Document;
use crate::ports::FileStore;

impl App {
    pub(super) fn close_tab(
        &mut self,
        source: WindowId,
        index: usize,
        discard: bool,
        store: &mut dyn FileStore,
    ) {
        let dirty = self
            .documents
            .get(index)
            .is_some_and(|slot| slot.document.is_dirty());
        if dirty && !discard {
            self.confirm_closing(source, index, None, store);
        } else {
            self.close_slot(index, store);
        }
    }

    /// Closes every clean tab, then asks about the first dirty one, `then` resuming
    /// afterwards; returns whether nothing is left open.
    pub(super) fn close_all(
        &mut self,
        source: WindowId,
        then: Intent,
        store: &mut dyn FileStore,
    ) -> bool {
        for index in (0..self.documents.len()).rev() {
            let clean = self
                .documents
                .get(index)
                .is_some_and(|slot| !slot.document.is_dirty());
            if clean {
                self.close_slot(index, store);
            }
        }
        if self.documents.is_empty() {
            return true;
        }
        self.confirm_closing(source, 0, Some(then), store);
        false
    }

    /// Brings tab `index` to the front and asks whether to save it before it closes; the
    /// dialog remembers the tab and runs `then` once it is closed.
    fn confirm_closing(
        &mut self,
        source: WindowId,
        index: usize,
        then: Option<Intent>,
        store: &mut dyn FileStore,
    ) {
        self.switch_tab(index, store);
        let title = self
            .documents
            .tabs()
            .get(index)
            .map_or_else(|| "this scene".to_owned(), |tab| tab.title.clone());
        let mut dialog = Dialog::new(DialogKind::Confirm);
        dialog.form.message = format!("Save changes to {title}?");
        *dialog.form.number(keys::TAB, 0) = i64::try_from(index).unwrap_or(i64::MAX);
        dialog.then = then.map(Box::new);
        self.window_mut(source, |window| {
            window.dialog = Some(dialog);
            window.ui.focus = None;
        });
        self.editing.status = format!("{title} has unsaved changes");
    }

    /// An untitled tab needs a native pick first; the Confirm dialog stays open until the
    /// pick lands (`resume_closing`) or the user chooses otherwise.
    pub(super) fn save_and_close(
        &mut self,
        source: WindowId,
        index: usize,
        then: Option<Box<Intent>>,
        input: &Input,
        store: &mut dyn FileStore,
    ) {
        self.switch_tab(index, store);
        if self.documents.active_index() != Some(index) {
            return;
        }
        if self.document().is_some_and(Document::is_untitled) {
            self.pick(source, PickPurpose::SaveAs);
            return;
        }
        self.save(store);
        self.finish_closing(source, index, then, input, store);
    }

    /// A pick can only be answered while a Confirm dialog is open when its Save asked for
    /// it, so a successful save then completes that dialog.
    pub(super) fn resume_closing(
        &mut self,
        source: WindowId,
        input: &Input,
        store: &mut dyn FileStore,
    ) {
        let pending = self
            .window(source)
            .and_then(|window| window.dialog.as_ref())
            .filter(|dialog| dialog.kind == DialogKind::Confirm)
            .and_then(|dialog| {
                let tab = usize::try_from(*dialog.form.numbers.get(keys::TAB)?).ok()?;
                Some((tab, dialog.then.clone()))
            });
        if let Some((index, then)) = pending {
            self.finish_closing(source, index, then, input, store);
        }
    }

    fn finish_closing(
        &mut self,
        source: WindowId,
        index: usize,
        then: Option<Box<Intent>>,
        input: &Input,
        store: &mut dyn FileStore,
    ) {
        let clean = self.documents.active_index() == Some(index)
            && self.document().is_some_and(|document| !document.is_dirty());
        if !clean {
            return;
        }
        self.window_mut(source, |window| window.dialog = None);
        self.close_slot(index, store);
        if let Some(then) = then {
            self.apply(*then, source, input, store);
        }
    }
}
