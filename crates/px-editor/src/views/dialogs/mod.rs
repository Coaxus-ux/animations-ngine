mod canvas_size;
mod confirm;
mod export;
mod new_doc;
mod properties;

use px_ui::{Area, Id, Rect, Ui, button, modal, rect};

use super::Context;
use crate::app::{Dialog, DialogKind, Intent};

pub(super) fn draw(ui: &mut Ui<'_>, context: &mut Context<'_>) {
    let Some(dialog) = context.dialog.take() else {
        return;
    };
    let (title, size) = match dialog.kind {
        DialogKind::New => (new_doc::TITLE, new_doc::SIZE),
        DialogKind::Export | DialogKind::ExportSheet => (export::TITLE, export::SIZE),
        DialogKind::CanvasSize => (canvas_size::TITLE, canvas_size::SIZE),
        DialogKind::Properties => (properties::TITLE, properties::SIZE),
        DialogKind::Confirm => (confirm::TITLE, confirm::SIZE),
    };
    let close = modal(ui, Id::new("dialog"), title, size, |ui, header, body| {
        let close = rect(header.right() - 14, header.y + 1, 12, header.height - 2);
        let closed = button(ui, Id::new("dialog-close"), close, "x");
        body_of(ui, context, dialog, body);
        closed
    });
    if close {
        context.intents.push(Intent::CloseDialog);
    }
}

fn body_of<'a>(ui: &mut Ui<'_>, context: &mut Context<'a>, dialog: &'a mut Dialog, body: Rect) {
    match dialog.kind {
        DialogKind::New => new_doc::draw(ui, context, dialog, body),
        DialogKind::Export | DialogKind::ExportSheet => export::draw(ui, context, dialog, body),
        DialogKind::CanvasSize => canvas_size::draw(ui, context, dialog, body),
        DialogKind::Properties => properties::draw(ui, context, dialog, body),
        DialogKind::Confirm => confirm::draw(ui, context, dialog, body),
    }
    context.dialog = Some(dialog);
}
