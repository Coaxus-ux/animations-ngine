use std::path::{Path, PathBuf};

use px_ui::{
    Area, Id, ROW, Rect, Ui, button, centered_label, label, list_row_icon, panel, rect, separator,
};

use super::Context;
use crate::app::{DialogKind, Intent};

const CARD_WIDTH: i32 = 200;
const BUTTON_HEIGHT: i32 = ROW + 6;
const GAP: i32 = 4;
const PAD: i32 = 6;
const MIN_LIST_ROWS: i32 = 4;
const FORGET_WIDTH: i32 = ROW;

/// Aseprite's Home tab: what to do when nothing is open, and the files opened lately.
pub(super) fn home(ui: &mut Ui<'_>, context: &mut Context<'_>, area: Rect) {
    let roles = ui.theme.roles;
    ui.canvas.fill(area, roles.panel_dark);
    let recent = context.recent;
    let rows = i32::try_from(recent.len())
        .unwrap_or(MIN_LIST_ROWS)
        .max(MIN_LIST_ROWS);
    let height = 2 * (BUTTON_HEIGHT + GAP) + ROW + GAP + rows * ROW + 3 * PAD;
    let card = area.centered(CARD_WIDTH, height).intersect(area);
    let (_, body) = panel(ui, card, "Start");
    let body = body.inset(PAD);
    let id = Id::new("home");

    let (new, rest) = body.take_top(BUTTON_HEIGHT);
    if button(ui, id.with_name("new"), new, "New file (Ctrl+N)") {
        context.intents.push(Intent::ShowDialog(DialogKind::New));
    }
    let (_, rest) = rest.take_top(GAP);
    let (open, rest) = rest.take_top(BUTTON_HEIGHT);
    if button(ui, id.with_name("open"), open, "Open file (Ctrl+O)") {
        context.intents.push(Intent::OpenFile(PathBuf::new()));
    }
    let (_, rest) = rest.take_top(GAP);
    separator(ui, rest.take_top(1).0);
    recent_list(ui, context, rest.take_top(GAP).1, recent);
}

fn recent_list(ui: &mut Ui<'_>, context: &mut Context<'_>, area: Rect, recent: &[PathBuf]) {
    let roles = ui.theme.roles;
    let id = Id::new("recent");
    let (heading, rest) = area.take_top(ROW);
    let (clear, caption) = heading.take_right(30);
    label(ui, caption, "Recent files", roles.ink_dim);
    if recent.is_empty() {
        centered_label(ui, rest, "Nothing opened yet", roles.ink_disabled);
        return;
    }
    if button(ui, id.with_name("clear"), clear, "Clear") {
        context.intents.push(Intent::ClearRecent);
    }
    let mut rows = rest;
    for (index, path) in recent.iter().enumerate() {
        let (row, rest) = rows.take_top(ROW);
        rows = rest;
        let (forget, row) = row.take_right(FORGET_WIDTH);
        let row_id = id.with(index as u64);
        if list_row_icon(ui, row_id, row, "file", &file_name(path), false).clicked {
            context.intents.push(Intent::OpenFile(path.clone()));
        }
        if ui.hovering(row) {
            ui.tooltip(&path.to_string_lossy());
        }
        let forget = rect(
            forget.x + 1,
            forget.y + 1,
            forget.width - 2,
            forget.height - 2,
        );
        if button(ui, row_id.with_name("forget"), forget, "x") {
            context.intents.push(Intent::ForgetRecent(path.clone()));
        }
    }
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.to_string_lossy().into_owned(),
        |name| name.to_string_lossy().into_owned(),
    )
}
