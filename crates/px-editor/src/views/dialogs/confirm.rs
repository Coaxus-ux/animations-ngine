use px_ui::{Area, Id, Key, ROW, Rect, Ui, button, centered_label, rect};

use super::Context;
use crate::app::{Dialog, Intent, keys};

pub(super) const TITLE: &str = "Save changes?";
pub(super) const SIZE: (i32, i32) = (220, 70);

const BUTTON_HEIGHT: i32 = ROW + 2;
const BUTTON_GAP: i32 = 4;
const BUTTONS: [&str; 3] = ["Save", "Don't save", "Cancel"];
const SAVE: usize = 0;
const DISCARD: usize = 1;
const CANCEL: usize = 2;

pub(super) fn draw(ui: &mut Ui<'_>, context: &mut Context<'_>, dialog: &mut Dialog, body: Rect) {
    let roles = ui.theme.roles;
    let (message, rest) = body.take_top(body.height - BUTTON_HEIGHT - BUTTON_GAP);
    centered_label(ui, message, &dialog.form.message, roles.ink);
    let row = rest.take_top(BUTTON_HEIGHT).0;
    let width = (row.width - 2 * BUTTON_GAP) / 3;
    let id = Id::new("confirm");
    let mut chosen = None;
    for (index, label) in BUTTONS.iter().enumerate() {
        let x = row.x + i32::try_from(index).unwrap_or(0) * (width + BUTTON_GAP);
        if button(
            ui,
            id.with_name(label),
            rect(x, row.y, width, row.height),
            label,
        ) {
            chosen = Some(index);
        }
    }
    if ui.input.key(Key::Enter) {
        chosen = Some(SAVE);
    }
    if ui.input.key(Key::Escape) {
        chosen = Some(CANCEL);
    }
    let Some(chosen) = chosen else { return };
    let tab = dialog
        .form
        .numbers
        .get(keys::TAB)
        .and_then(|tab| usize::try_from(*tab).ok());
    match (chosen, tab) {
        (SAVE, Some(index)) => context.intents.push(Intent::SaveAndClose {
            index,
            then: dialog.then.clone(),
        }),
        (DISCARD, Some(index)) => {
            context.intents.push(Intent::CloseDialog);
            context.intents.push(Intent::CloseTab {
                index,
                discard: true,
            });
            if let Some(then) = &dialog.then {
                context.intents.push((**then).clone());
            }
        }
        _ => context.intents.push(Intent::CloseDialog),
    }
}
