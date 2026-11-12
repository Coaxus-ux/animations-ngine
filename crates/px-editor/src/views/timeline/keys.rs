use px_ui::{Id, Rect, Ui, rect};

use super::Context;
use super::columns::{CELL, Columns, name_cell, visible};
use crate::app::Intent;
use crate::domain::timeline::KeyRow;

pub(super) fn row(
    ui: &mut Ui<'_>,
    context: &mut Context<'_>,
    at: (Rect, i32),
    columns: Columns,
    (index, key_row): (usize, &KeyRow),
) {
    if !visible(at) {
        return;
    }
    let roles = ui.theme.roles;
    let selected = key_row.path.is_some() && context.selected == key_row.path.as_ref();
    let name = name_cell(ui, at, &key_row.label, selected);
    let id = Id::new("timeline").with_name("keys").with(index as u64);
    if ui.interact(id, name).clicked {
        context
            .intents
            .push(Intent::SelectElement(key_row.path.clone()));
    }
    let middle = at.1 + CELL / 2;
    if let (Some(first), Some(last)) = (key_row.frames.first(), key_row.frames.last()) {
        let from = columns.center((*first).max(columns.first));
        let to = columns.center((*last).min(columns.first + columns.count - 1));
        if first < last && to > from {
            ui.canvas
                .fill(rect(from, middle, to - from, 1), roles.ink_disabled);
        }
    }
    let range = columns.frames();
    for &frame in key_row.frames.iter().filter(|frame| range.contains(frame)) {
        let cell = rect(columns.x_of(frame), at.1, columns.cell, CELL);
        let response = ui.interact(id.with(u64::from(frame)), cell);
        if response.clicked {
            context
                .intents
                .push(Intent::SelectElement(key_row.path.clone()));
            context.intents.push(Intent::Frame(frame));
        }
        if response.hovered {
            ui.tooltip(&format!("{} · frame {}", key_row.label, frame + 1));
        }
        let ink = if selected || response.hovered {
            roles.accent_light
        } else {
            roles.ink
        };
        let x = columns.center(frame);
        ui.canvas.fill(rect(x - 1, middle - 3, 3, 7), ink);
        ui.canvas.fill(rect(x - 2, middle - 2, 5, 5), ink);
        ui.canvas.fill(rect(x - 3, middle - 1, 7, 3), ink);
    }
}
