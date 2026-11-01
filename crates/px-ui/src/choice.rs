use crate::area::{Area, Rect};
use crate::input::Key;
use crate::ui::{Id, Ui};
use crate::widgets::centered_label;

pub fn segmented(ui: &mut Ui<'_>, id: Id, area: Rect, labels: &[&str], chosen: &mut usize) -> bool {
    let roles = ui.theme.roles;
    let before = *chosen;
    let focused = ui.focusable(id);
    if focused && !labels.is_empty() {
        if ui.take_key(Key::Right) {
            *chosen = (*chosen + 1).min(labels.len() - 1);
        }
        if ui.take_key(Key::Left) {
            *chosen = chosen.saturating_sub(1);
        }
    }
    let count = i32::try_from(labels.len()).unwrap_or(0).max(1);
    ui.canvas.fill(area, roles.panel_dark);
    let mut rest = area.inset(1);
    for (index, text) in labels.iter().enumerate() {
        let remaining = count - i32::try_from(index).unwrap_or(0);
        let (cell, after) = rest.take_left(rest.width / remaining.max(1));
        rest = after;
        let response = ui.interact(id.with(index as u64), cell);
        if response.clicked {
            *chosen = index;
        }
        let selected = *chosen == index;
        if selected {
            ui.canvas.fill(cell, roles.accent);
            ui.canvas
                .bevel(cell, roles.panel_light, roles.panel_dark, true);
        } else {
            ui.canvas.fill(cell, roles.panel);
            if response.hovered {
                ui.canvas.dither(cell, roles.panel_light);
            }
        }
        let ink = if selected { roles.ink } else { roles.ink_dim };
        centered_label(ui, cell, text, ink);
    }
    if focused {
        ui.focus_ring(area);
    }
    *chosen != before
}
