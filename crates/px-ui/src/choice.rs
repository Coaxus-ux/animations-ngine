use crate::area::{Area, Rect};
use crate::ui::{Id, Ui};
use crate::widgets::centered_label;

/// One choice among `labels`, drawn as equal cells in a row; the chosen cell is accented.
/// Returns whether `chosen` changed.
pub fn segmented(ui: &mut Ui<'_>, id: Id, area: Rect, labels: &[&str], chosen: &mut usize) -> bool {
    let roles = ui.theme.roles;
    let before = *chosen;
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
        let face = if selected {
            roles.accent
        } else if response.hovered {
            roles.panel_light
        } else {
            roles.panel
        };
        ui.canvas.fill(cell, face);
        let ink = if selected { roles.ink } else { roles.ink_dim };
        centered_label(ui, cell, text, ink);
    }
    *chosen != before
}
