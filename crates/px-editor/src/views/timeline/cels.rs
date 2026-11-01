use px_engine::{Geometry, Scene};
use px_ui::{Area, Id, Rect, Ui, rect};

use super::Context;
use super::columns::{CELL, Columns, name_cell, visible};
use crate::app::Intent;
use crate::domain::{Document, Layer};

pub(super) fn rows(document: &Document) -> Vec<(usize, &Layer)> {
    document
        .layers()
        .iter()
        .enumerate()
        .filter(|(_, layer)| layer.art.is_some())
        .rev()
        .collect()
}

pub(super) fn row(
    ui: &mut Ui<'_>,
    context: &mut Context<'_>,
    at: (Rect, i32),
    columns: Columns,
    (scene, (index, layer)): (&Scene, (usize, &Layer)),
) {
    if !visible(at) {
        return;
    }
    let roles = ui.theme.roles;
    let selected = context.view.layer == Some(index);
    name_cell(ui, at, &layer.name, selected);
    let Some(art) = &layer.art else { return };
    let Some(Geometry::Pixels(pixels)) = scene.shapes().get(art.shape).map(|shape| &shape.geometry)
    else {
        return;
    };
    let id = Id::new("timeline").with_name("cels").with(index as u64);
    let frame = context.view.frame % scene.timing().duration.get().max(1);
    for f in columns.frames() {
        let cell = rect(columns.x_of(f), at.1, columns.cell, CELL);
        let response = ui.interact(id.with(u64::from(f)), cell);
        if response.clicked {
            context.intents.push(Intent::Frame(f));
            context.intents.push(Intent::SelectLayer(index));
        }
        let ink = if f == frame && selected {
            roles.accent_light
        } else {
            roles.ink
        };
        if pixels.cels.iter().any(|cel| cel.frame == f) {
            ui.canvas.fill(cell.inset(1), ink);
        } else if pixels.cel_at(f).is_some() {
            ui.canvas.fill(
                rect(cell.x, cell.y + CELL / 2 - 1, columns.cell, 2),
                roles.ink_dim,
            );
        }
        if response.hovered {
            ui.canvas.frame(cell, roles.accent_light);
        }
    }
}
