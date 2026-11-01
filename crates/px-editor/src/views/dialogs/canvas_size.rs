use px_core::MAX_CANVAS_SIDE;
use px_ui::{
    Area, DialogAction, Id, ROW, Rect, Ui, focus_next, icon_button, label, number_field, ok_cancel,
    rect,
};

use super::Context;
use crate::app::{Dialog, Intent, keys};
use crate::domain::{Anchor, Document};

pub(super) const TITLE: &str = "Canvas Size";
pub(super) const SIZE: (i32, i32) = (240, 120);

const FIELD: i32 = ROW + 2;
const GAP: i32 = 3;
const CAPTION: i32 = 36;
/// The dialog body's inset on each side, in UI pixels.
const INSET: i32 = 4;
/// One cell of the anchor grid, in UI pixels.
const CELL: i32 = 11;
const NOTE: &str = "Pixels outside the new canvas are dropped";

/// Width and height fields, the anchor that stays put, and Resize.
pub(super) fn draw(ui: &mut Ui<'_>, context: &mut Context<'_>, dialog: &mut Dialog, body: Rect) {
    let roles = ui.theme.roles;
    let id = Id::new("canvas-size");
    let body = body.inset(INSET);
    let (buttons, rest) = body.take_bottom(FIELD);
    let (note, rest) = rest.take_bottom(ROW);
    let (grid, fields) = rest.take_right(3 * CELL + 8);
    let range = (1, i64::from(MAX_CANVAS_SIDE));
    let width_id = id.with_name(keys::WIDTH);
    let height_id = id.with_name(keys::HEIGHT);
    let (width_row, rest) = fields.take_top(FIELD);
    let height_row = rest.take_top(GAP).1.take_top(FIELD).0;
    for (row, caption, field_id, key) in [
        (width_row, " Width", width_id, keys::WIDTH),
        (height_row, " Height", height_id, keys::HEIGHT),
    ] {
        let (caption_area, field) = row.take_left(CAPTION);
        label(ui, caption_area, caption, roles.ink_dim);
        number_field(ui, field_id, field, dialog.form.number(key, 1), range);
    }
    focus_next(ui, &[width_id, height_id]);
    let chosen = dialog.form.choice(keys::ANCHOR, Anchor::default_index());
    anchor_grid(ui, id.with_name(keys::ANCHOR), grid, chosen);
    let anchor = Anchor::ALL.get(*chosen).copied().unwrap_or_default();
    label(ui, note, NOTE, roles.ink_dim);
    let width = *dialog.form.number(keys::WIDTH, 1);
    let height = *dialog.form.number(keys::HEIGHT, 1);
    let current = context.document.and_then(Document::scene).map(|scene| {
        (
            i64::from(scene.size().width()),
            i64::from(scene.size().height()),
        )
    });
    let changed = current != Some((width, height));
    match ok_cancel(ui, id, buttons, "Resize", changed) {
        DialogAction::Ok => {
            context.intents.push(Intent::ResizeCanvas {
                width: u16::try_from(width).unwrap_or(1),
                height: u16::try_from(height).unwrap_or(1),
                anchor,
            });
            context.intents.push(Intent::CloseDialog);
        }
        DialogAction::Cancel => context.intents.push(Intent::CloseDialog),
        DialogAction::None => {}
    }
}

/// Nine buttons laid out like the canvas's corners, edges and middle; the chosen one is
/// marked.
fn anchor_grid(ui: &mut Ui<'_>, id: Id, area: Rect, chosen: &mut usize) {
    let roles = ui.theme.roles;
    let (caption, rest) = area.take_top(ROW);
    label(ui, caption, " Anchor", roles.ink_dim);
    let origin = (rest.x + 4, rest.y + 1);
    for (index, anchor) in Anchor::ALL.into_iter().enumerate() {
        let (column, row) = anchor.cell();
        let cell = rect(
            origin.0 + i32::from(column) * CELL,
            origin.1 + i32::from(row) * CELL,
            CELL - 1,
            CELL - 1,
        );
        let selected = *chosen == index;
        if icon_button(ui, id.with(index as u64), cell, "", selected, "") {
            *chosen = index;
        }
        if selected {
            ui.canvas.fill(cell.inset(3), roles.accent_light);
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests fail loudly")]

    use px_format::Font;
    use px_ui::text_width;

    use super::*;

    #[test]
    fn the_note_fits_the_dialog() {
        let font = Font::builtin().unwrap();
        assert!(text_width(&font, NOTE) <= SIZE.0 - 2 * INSET);
    }
}
