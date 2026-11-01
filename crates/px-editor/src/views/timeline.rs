use px_engine::Geometry;
use px_ui::{Area, Id, ROW, Rect, Ui, button, checkbox, label, rect, scroll_area, text_width};

use super::{Context, unavailable};
use crate::app::Intent;

const CELL: i32 = 8;
const NAME: i32 = 44;
const BUTTON: i32 = ROW + 1;

pub(super) fn timeline(ui: &mut Ui<'_>, context: &mut Context<'_>, area: Rect) {
    let Some((document, scene)) = context
        .document
        .and_then(|document| Some((document, document.scene()?)))
    else {
        unavailable(ui, context, area);
        return;
    };
    let roles = ui.theme.roles;
    let frames = scene.timing().duration.get().max(1);
    let frame = context.view.frame % frames;
    let (bar, grid) = area.take_top(BUTTON + 2);
    controls(ui, context, bar, frame, frames);

    let id = Id::new("timeline");
    let rows: Vec<(usize, &crate::domain::Layer)> = document
        .layers()
        .iter()
        .enumerate()
        .filter(|(_, layer)| layer.art.is_some())
        .rev()
        .collect();
    let rows_height = i32::try_from(rows.len()).unwrap_or(0) * (CELL + 2);
    let (grid, offset) = scroll_area(ui, id.with_name("rows"), grid, rows_height);
    let clip = ui.canvas.clip_to(grid);
    let (names, cells) = grid.take_left(NAME);
    let visible = u32::try_from((cells.width / CELL).max(1)).unwrap_or(1);
    let first = frame
        .saturating_sub(visible / 2)
        .min(frames.saturating_sub(visible));
    let column = |f: u32| cells.x + i32::try_from(f - first).unwrap_or(0) * CELL;
    let current = rect(column(frame), cells.y, CELL, cells.height);
    ui.canvas.fill(current, roles.panel_light);
    for (slot, (index, layer)) in (0i32..).zip(rows) {
        let y = grid.y + slot * (CELL + 2) - offset;
        if y + CELL < grid.y || y > grid.bottom() {
            continue;
        }
        let selected = context.view.layer == Some(index);
        let name_row = rect(names.x, y, names.width, CELL + 1);
        if selected {
            ui.canvas.fill(name_row, roles.accent);
        }
        label(
            ui,
            name_row,
            &format!(" {}", layer.name),
            if selected { roles.ink } else { roles.ink_dim },
        );
        let Some(art) = &layer.art else { continue };
        let Some(Geometry::Pixels(pixels)) =
            scene.shapes().get(art.shape).map(|shape| &shape.geometry)
        else {
            continue;
        };
        for f in first..(first + visible).min(frames) {
            let cell = rect(column(f), y, CELL, CELL);
            let response = ui.interact(id.with(index as u64).with(u64::from(f)), cell);
            if response.clicked {
                context.intents.push(Intent::Frame(f));
                context.intents.push(Intent::SelectLayer(index));
            }
            let starts = pixels.cels.iter().any(|cel| cel.frame == f);
            let shown = pixels.cel_at(f).is_some();
            let ink = if f == frame && selected {
                roles.accent_light
            } else {
                roles.ink
            };
            if starts {
                ui.canvas.fill(cell.inset(1), ink);
            } else if shown {
                ui.canvas
                    .fill(rect(cell.x, cell.y + CELL / 2 - 1, CELL, 2), roles.ink_dim);
            }
            if response.hovered {
                ui.canvas.frame(cell, roles.accent_light);
            }
        }
    }
    ui.canvas.restore_clip(clip);
}

fn controls(ui: &mut Ui<'_>, context: &mut Context<'_>, bar: Rect, frame: u32, frames: u32) {
    let roles = ui.theme.roles;
    let id = Id::new("timeline").with_name("controls");
    let playing = context.editing.playing;
    let buttons = [
        (
            if playing { "Pause" } else { "Play" },
            "Enter",
            Intent::TogglePlay,
        ),
        (
            "New cel",
            "A copy of what shows at this frame",
            Intent::NewCel,
        ),
        (
            "Delete cel",
            "Remove the cel starting at this frame",
            Intent::DeleteCel,
        ),
        ("+ Frame", "Make the scene a frame longer", Intent::AddFrame),
        (
            "- Frame",
            "Make the scene a frame shorter",
            Intent::DeleteFrame,
        ),
    ];
    let mut x = bar.x + 1;
    for (text, tip, intent) in buttons {
        let width = text_width(ui.font, text) + 10;
        let at = rect(x, bar.y + 1, width, BUTTON);
        if button(ui, id.with_name(text), at, text) {
            context.intents.push(intent);
        }
        if ui.hovering(at) {
            ui.tooltip(tip);
        }
        x += width + 2;
    }
    let onion = rect(x, bar.y + 1, 52, BUTTON);
    if checkbox(
        ui,
        id.with_name("onion"),
        onion,
        "Onion",
        context.view.onion,
    ) {
        context.intents.push(Intent::ToggleOnion);
    }
    x += 56;
    let text = format!("frame {}/{frames}", frame + 1);
    label(
        ui,
        rect(x + 2, bar.y, bar.right() - x, BUTTON + 2),
        &text,
        roles.ink_dim,
    );
}
