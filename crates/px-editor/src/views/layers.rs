use px_engine::Param;
use px_ui::{
    Area, Id, ROW, Rect, Ui, button, drop_index, label, list_row, rect, scroll_area, slider,
    text_field, toggle_icon,
};

use super::Context;
use crate::app::Intent;
use crate::domain::{Document, Layer};

const BUTTONS: i32 = 2 * (ROW + 2);

/// Layer rows, top layer first (drawn last): show/hide and lock toggles, the name (double
/// click renames), drag to reorder; below, opacity and add, delete, move buttons.
pub(super) fn layers(ui: &mut Ui<'_>, context: &mut Context<'_>, area: Rect) {
    let Some(document) = context.document else {
        return;
    };
    let rows = document.layers();
    let (buttons, rest) = area.take_bottom(BUTTONS);
    let (opacity_row, list) = rest.take_bottom(ROW + 2);
    let id = Id::new("layers");
    let total_height = i32::try_from(rows.len()).unwrap_or(0) * ROW;
    let (view, offset) = scroll_area(ui, id, list, total_height);
    let clip = ui.canvas.clip_to(view);
    let order: Vec<usize> = (0..rows.len()).rev().collect();
    let top = view.y - offset;
    let mut dragging = false;
    for (slot, &index) in (0i32..).zip(&order) {
        let row = rect(view.x, top + slot * ROW, view.width, ROW);
        if row.bottom() >= view.y && row.y <= view.bottom() {
            let drag = layer_row(
                ui,
                context,
                id.with(index as u64),
                (index, &rows[index]),
                row,
            );
            dragging |= drag.held;
            if drag.released {
                let target = drop_index(
                    top,
                    ROW,
                    rows.len().saturating_sub(1),
                    ui.input.pointer_anywhere.1,
                );
                if let Some(&to) = order.get(target).filter(|to| **to != index) {
                    context.intents.push(Intent::MoveLayer { from: index, to });
                }
            }
        }
    }
    if dragging {
        let target = drop_index(top, ROW, rows.len(), ui.input.pointer_anywhere.1);
        let y = top + i32::try_from(target).unwrap_or(0) * ROW;
        ui.canvas
            .fill(rect(view.x, y, view.width, 1), ui.theme.roles.accent_light);
    }
    ui.canvas.restore_clip(clip);
    opacity(ui, context, document, opacity_row);
    actions(ui, context, buttons);
}

/// Whether a row is being dragged far enough to reorder, and whether it was let go.
#[derive(Clone, Copy, Default)]
struct Drag {
    held: bool,
    released: bool,
}

fn layer_row(
    ui: &mut Ui<'_>,
    context: &mut Context<'_>,
    id: Id,
    (index, layer): (usize, &Layer),
    row: Rect,
) -> Drag {
    let (eye, rest) = row.take_left(ROW);
    let (lock, name) = rest.take_left(ROW);
    let hidden = context.view.hidden.contains(&layer.name);
    if toggle_icon(
        ui,
        id.with_name("eye"),
        eye,
        !hidden,
        ("eye", "eye-off"),
        "Show / hide in this window",
    ) {
        context
            .intents
            .push(Intent::ToggleHidden(layer.name.clone()));
    }
    let locked = context.editing.locked.contains(&layer.name);
    if layer.paintable()
        && toggle_icon(
            ui,
            id.with_name("lock"),
            lock,
            locked,
            ("lock", "unlock"),
            "Lock painting",
        )
    {
        context
            .intents
            .push(Intent::ToggleLocked(layer.name.clone()));
    }
    if context.renaming == Some(index) {
        let field = id.with_name("rename");
        let text = context
            .fields
            .entry(field)
            .or_insert_with(|| layer.name.clone());
        ui.state.focus = Some(field);
        if text_field(ui, field, name, text).submitted {
            let name = context.fields.remove(&field).unwrap_or_default();
            context
                .intents
                .push(Intent::RenameLayer { layer: index, name });
        }
        return Drag::default();
    }
    let kind = if layer.paintable() { "" } else { "  (shapes)" };
    let selected = context.view.layer == Some(index);
    let response = list_row(ui, id, name, &format!("{}{kind}", layer.name), selected);
    if response.clicked {
        context.intents.push(Intent::SelectLayer(index));
        if response.double_clicked {
            context.intents.push(Intent::StartRename(index));
        }
    }
    let far = response.drag.1.abs() > ROW / 2;
    Drag {
        held: response.held && far,
        released: response.released && far,
    }
}

fn opacity(ui: &mut Ui<'_>, context: &mut Context<'_>, document: &Document, area: Rect) {
    let Some(index) = context.view.layer else {
        return;
    };
    let Some(art) = document
        .layers()
        .get(index)
        .and_then(|layer| layer.art.as_ref())
    else {
        return;
    };
    let Some(scene) = document.scene() else {
        return;
    };
    let current = match scene
        .shapes()
        .get(art.shape)
        .map(|shape| &shape.style.opacity)
    {
        Some(Param::Fixed(value)) => *value,
        _ => 1.0,
    };
    let (caption, bar) = area.inset(1).take_left(44);
    label(ui, caption, " Opacity", ui.theme.roles.ink_dim);
    let field = Id::new("layers").with_name("opacity");
    let mut value = context
        .fields
        .get(&field)
        .and_then(|text| text.parse().ok())
        .unwrap_or(current);
    if slider(ui, field, bar, &mut value, (0.0, 1.0), 0.05) {
        context.fields.insert(field, value.to_string());
    }
    if ui.input.released.left && context.fields.remove(&field).is_some() {
        context.intents.push(Intent::Opacity {
            layer: index,
            value,
        });
    }
}

/// Labeled buttons for what can be done to layers, in two rows.
fn actions(ui: &mut Ui<'_>, context: &mut Context<'_>, area: Rect) {
    let id = Id::new("layers").with_name("actions");
    let active = context.view.layer;
    let rows: [&[(&str, Option<Intent>)]; 2] = [
        &[
            ("New", Some(Intent::AddLayer)),
            ("Rename", active.map(Intent::StartRename)),
            ("Delete", active.map(Intent::DeleteLayer)),
        ],
        &[
            ("Move up", move_by(context, 1)),
            ("Move down", move_by(context, -1)),
        ],
    ];
    let (top, bottom) = area.take_top(area.height / 2);
    for (row, buttons) in [top, bottom].into_iter().zip(rows) {
        let count = i32::try_from(buttons.len()).unwrap_or(1);
        let width = row.width / count;
        for (slot, (text, intent)) in (0i32..).zip(buttons) {
            let at = rect(row.x + slot * width, row.y, width - 1, row.height - 1);
            if button(ui, id.with_name(text), at, text)
                && let Some(intent) = intent.clone()
            {
                context.intents.push(intent);
            }
        }
    }
}

/// Moving the active layer `by` rows up (towards the top of the panel) or down.
fn move_by(context: &Context<'_>, by: i64) -> Option<Intent> {
    let from = context.view.layer?;
    let count = context.document?.layers().len();
    let to = usize::try_from(i64::try_from(from).ok()? + by).ok()?;
    (to < count).then_some(Intent::MoveLayer { from, to })
}
