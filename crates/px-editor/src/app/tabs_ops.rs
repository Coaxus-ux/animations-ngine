use px_format::edit::NodePath;
use px_ui::Input;

use super::editing::Editing;
use super::intent::Intent;
use super::window::{ViewState, WindowKind};
use super::{App, Targets};
use crate::domain::{Command, Document};
use crate::ports::FileStore;

impl App {
    pub(super) fn apply_tabs(
        &mut self,
        intent: Intent,
        _targets: Targets,
        _input: &Input,
        store: &mut dyn FileStore,
    ) -> Option<Intent> {
        match intent {
            Intent::ActivateTab(index) => self.switch_tab(index, store),
            Intent::GoHome => {
                if self.documents.active_index().is_some() {
                    self.leave_active(store);
                    self.park_active();
                    self.documents.deactivate();
                    self.editing = super::Editing::default();
                    self.retitle();
                }
            }
            other => return Some(other),
        }
        None
    }

    pub(super) fn add_document(&mut self, document: Document, store: &mut dyn FileStore) {
        self.leave_active(store);
        self.park_active();
        let primary = first_paint(&document);
        let selection = document.scene().map(|scene| {
            let size = scene.size();
            crate::domain::Selection::none(i32::from(size.width()), i32::from(size.height()))
        });
        let first_layer = document
            .layers()
            .iter()
            .position(crate::domain::Layer::paintable);
        self.documents.open(document);
        self.editing = Editing {
            primary,
            selection: selection.unwrap_or_default(),
            ..Editing::default()
        };
        for window in &mut self.windows {
            if window.kind == WindowKind::Main {
                window.view = ViewState {
                    layer: first_layer,
                    ..ViewState::default()
                };
            } else {
                window.view.layer = first_layer;
                window.view.hidden.clear();
            }
        }
        self.retitle();
    }

    pub(super) fn switch_tab(&mut self, index: usize, store: &mut dyn FileStore) {
        if self.documents.active_index() == Some(index) || index >= self.documents.len() {
            return;
        }
        self.leave_active(store);
        self.park_active();
        self.documents.activate(index);
        self.bring_in();
        self.retitle();
    }

    pub(super) fn close_slot(&mut self, index: usize, store: &mut dyn FileStore) {
        let was_active = self.documents.active_index() == Some(index);
        if was_active {
            self.leave_active(store);
        }
        if self.documents.close(index).is_none() {
            return;
        }
        if was_active {
            self.bring_in();
        }
        self.retitle();
    }

    pub(super) fn leave_active(&mut self, store: &mut dyn FileStore) {
        let mut edits = Vec::new();
        for window in &mut self.windows {
            let code = core::mem::take(&mut window.code);
            let selected = window.selected.take();
            if code.edited_at.is_some() {
                edits.push((code.text, selected));
            }
        }
        let stroke = self.stroke.take().is_some();
        let Some(document) = self.document_mut() else {
            return;
        };
        if stroke {
            document.cancel_preview();
        }
        let mut refused = None;
        for (text, selected) in edits {
            if let Err(error) = flush_edit(document, text, selected, store) {
                refused = Some(error);
            }
        }
        if let Some(error) = refused {
            self.editing.status = error;
        }
    }

    fn bring_in(&mut self) {
        let Some(slot) = self.documents.active_mut() else {
            self.editing = Editing::default();
            return;
        };
        let editing = core::mem::take(&mut slot.editing);
        let view = core::mem::take(&mut slot.view);
        self.editing = editing;
        if let Some(main) = self.main_mut() {
            main.view = view;
        }
    }

    pub(super) fn park_active(&mut self) {
        let view = self
            .windows
            .iter()
            .find(|window| window.kind == WindowKind::Main)
            .map(|window| window.view.clone());
        let editing = self.editing.clone();
        if let Some(slot) = self.documents.active_mut() {
            slot.editing = editing;
            if let Some(view) = view {
                slot.view = view;
            }
        }
    }
}

fn flush_edit(
    document: &mut Document,
    text: String,
    selected: Option<NodePath>,
    store: &mut dyn FileStore,
) -> Result<(), String> {
    let node = selected.filter(|path| document.source().node_text(path).is_some());
    let Some(path) = node else {
        document.set_text(text, store);
        return Ok(());
    };
    document.apply(&Command::ReplaceNode { path, text }, store)
}

fn first_paint(document: &Document) -> Option<px_core::Paint> {
    let scene = document.scene()?;
    let names = &document.loaded()?.paint_names;
    let (_, ramp) = names.ramps().next()?;
    let tone = scene.palette().ramp(ramp).ok()?.len().saturating_sub(1);
    scene.palette().paint(ramp, px_core::Tone::new(tone)).ok()
}
