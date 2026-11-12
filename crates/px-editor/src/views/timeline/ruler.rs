use px_ui::{Area, Id, Rect, Ui, label, rect, text_width};

use super::Context;
use super::columns::{CELL, Columns, NAME, ROW_HEIGHT, name_cell, visible};
use crate::app::Intent;
use crate::domain::timeline::MarkerAt;

pub(super) fn ruler(
    ui: &mut Ui<'_>,
    context: &mut Context<'_>,
    area: Rect,
    columns: Columns,
    (frame, fps): (u32, u16),
) {
    let roles = ui.theme.roles;
    ui.canvas.fill(area, roles.panel_dark);
    ui.canvas.fill(
        rect(columns.x_of(frame), area.y, columns.cell, ROW_HEIGHT),
        roles.accent,
    );
    let seconds = f64::from(frame) / f64::from(fps.max(1));
    label(
        ui,
        rect(area.x, area.y, NAME, ROW_HEIGHT),
        &format!(" {seconds:.1}s"),
        roles.ink_dim,
    );
    let fps = u32::from(fps.max(1));
    let bottom = area.bottom();
    for f in columns.frames() {
        let x = columns.x_of(f);
        let (height, ink) = if f % fps == 0 {
            (ROW_HEIGHT, roles.ink_dim)
        } else if f % fps.div_ceil(2).max(1) == 0 {
            (4, roles.ink_disabled)
        } else {
            (2, roles.ink_disabled)
        };
        ui.canvas.fill(rect(x, bottom - height, 1, height), ink);
        if f % fps == 0 {
            let text = format!("{}s", f / fps);
            let room = (columns.right() - x - 2).min(text_width(ui.font, &text) + 2);
            label(
                ui,
                rect(x + 2, area.y, room, ROW_HEIGHT - 2),
                &text,
                roles.ink_dim,
            );
        }
    }
    let scrub = rect(
        columns.cells.x,
        area.y,
        columns.right() - columns.cells.x,
        ROW_HEIGHT,
    );
    let response = ui.interact(Id::new("timeline").with_name("ruler"), scrub);
    if response.pressed || response.held {
        let target = columns.frame_at(ui.input.pointer_anywhere.0);
        if target != frame {
            context.intents.push(Intent::Frame(target));
        }
    }
    if response.hovered {
        let (x, _) = ui.input.pointer_anywhere;
        let hovered = columns.frame_at(x);
        let at = f64::from(hovered) / f64::from(fps);
        ui.tooltip(&format!("frame {} · {at:.2}s", hovered + 1));
    }
}

pub(super) fn markers(
    ui: &mut Ui<'_>,
    context: &mut Context<'_>,
    at: (Rect, i32),
    columns: Columns,
    marks: &[MarkerAt],
) {
    if !visible(at) {
        return;
    }
    let roles = ui.theme.roles;
    name_cell(ui, at, "markers", false);
    let range = columns.frames();
    let id = Id::new("timeline").with_name("markers");
    for (index, mark) in marks.iter().enumerate() {
        if !range.contains(&mark.frame) {
            continue;
        }
        let x = columns.x_of(mark.frame);
        let next = marks
            .iter()
            .filter(|other| other.frame > mark.frame)
            .map(|other| columns.x_of(other.frame.min(range.end)))
            .min()
            .unwrap_or_else(|| columns.right());
        let area = rect(x, at.1, (next - x).max(columns.cell), CELL + 1);
        let response = ui.interact(id.with(index as u64), area);
        if response.clicked {
            context.intents.push(Intent::Frame(mark.frame));
        }
        if response.hovered {
            ui.tooltip(&format!("{} · frame {}", mark.name, mark.frame + 1));
        }
        let ink = if response.hovered {
            roles.accent_light
        } else {
            roles.accent
        };
        ui.canvas.fill(rect(x, at.1, 1, CELL + 1), ink);
        ui.canvas.fill(rect(x + 1, at.1, 3, 3), ink);
        label(
            ui,
            rect(x + 5, at.1, area.width - 6, CELL + 1),
            &mark.name,
            roles.ink,
        );
    }
}
