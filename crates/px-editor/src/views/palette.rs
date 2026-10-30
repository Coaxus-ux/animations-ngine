use px_core::{Paint, Rgb};
use px_ui::{Area, Id, ROW, Rect, Ui, button, label, rect, scroll_area, slider, text_field};

use super::Context;
use crate::app::Intent;

const SWATCH: i32 = 9;
const NAME: i32 = 34;

/// The document's palette: one row of swatches per ramp. Click draws with a paint (or,
/// with the pick tool, recolors the picked object); right click sets the second paint;
/// double click edits the color.
pub(super) fn palette(ui: &mut Ui<'_>, context: &mut Context<'_>, area: Rect) {
    let Some(document) = context.document else {
        return;
    };
    let Some(loaded) = document.loaded() else {
        return;
    };
    let palette = loaded.scene.palette();
    let roles = ui.theme.roles;
    let (current, rest) = area.take_top(px_ui::ROW + 4);
    actions(ui, context, current);
    let (swatches, picker_area) = match context.picker {
        Some(_) => {
            let (picker, swatches) = rest.take_bottom(5 * ROW + 6);
            (swatches, Some(picker))
        }
        None => (rest, None),
    };
    let ramps: Vec<(String, px_core::RampId)> = loaded
        .paint_names
        .ramps()
        .map(|(name, id)| (name.to_owned(), id))
        .collect();
    let id = Id::new("palette");
    let total_height = i32::try_from(ramps.len()).unwrap_or(0) * (SWATCH + 2);
    let (view, offset) = scroll_area(ui, id, swatches, total_height);
    let clip = ui.canvas.clip_to(view);
    for (row, (name, ramp)) in (0i32..).zip(&ramps) {
        let y = view.y + row * (SWATCH + 2) - offset;
        label(ui, rect(view.x, y, NAME, SWATCH + 1), name, roles.ink_dim);
        let colors = palette
            .ramp(*ramp)
            .map(|ramp| ramp.colors().to_vec())
            .unwrap_or_default();
        for (tone, color) in (0u8..).zip(colors) {
            let cell = rect(
                view.x + NAME + i32::from(tone) * (SWATCH + 1),
                y,
                SWATCH,
                SWATCH,
            );
            let paint = Paint {
                ramp: *ramp,
                tone: px_core::Tone::new(tone),
            };
            swatch(
                ui,
                context,
                id.with(u64::from(row.unsigned_abs())).with(u64::from(tone)),
                cell,
                paint,
                color,
            );
        }
    }
    ui.canvas.restore_clip(clip);
    if let (Some(picker), Some(area)) = (context.picker, picker_area) {
        color_picker(ui, context, area, picker.paint, picker.color);
    }
}

/// Buttons that act on the main color: change it in the palette, or give it to the
/// picked shape.
fn actions(ui: &mut Ui<'_>, context: &mut Context<'_>, area: Rect) {
    let id = Id::new("palette").with_name("actions");
    let (edit, recolor) = area.inset(1).take_left(area.width / 2);
    if button(
        ui,
        id.with_name("edit"),
        edit.take_left(edit.width - 1).0,
        "Edit color",
    ) {
        context.intents.push(Intent::EditPrimaryColor);
    }
    let picked = context.editing.picked.is_some();
    if button(ui, id.with_name("recolor"), recolor, "Recolor") {
        context.intents.push(if picked {
            Intent::RecolorPicked
        } else {
            Intent::Tool(crate::tools::ToolKind::Pick)
        });
    }
    if ui.hovering(recolor) {
        ui.tooltip(if picked {
            "Give the picked shape the main color"
        } else {
            "Choose the pick tool, then click a shape on the canvas"
        });
    }
}

fn swatch(
    ui: &mut Ui<'_>,
    context: &mut Context<'_>,
    id: Id,
    cell: Rect,
    paint: Paint,
    color: Rgb,
) {
    let roles = ui.theme.roles;
    let response = ui.interact(id, cell);
    ui.canvas.rgb_box(cell, color);
    if context.editing.primary == Some(paint) {
        ui.canvas.frame(
            rect(cell.x - 1, cell.y - 1, cell.width + 2, cell.height + 2),
            roles.accent_light,
        );
    } else if response.hovered {
        ui.canvas.frame(
            rect(cell.x - 1, cell.y - 1, cell.width + 2, cell.height + 2),
            roles.ink,
        );
    }
    if response.clicked {
        if super::double_click(context, id, ui.input.time_ms) {
            context.intents.push(Intent::OpenPicker(paint));
        } else {
            context.intents.push(Intent::Swatch(paint));
        }
    }
    if response.right_clicked {
        context.intents.push(Intent::SecondaryPaint(Some(paint)));
    }
}

/// Red, green and blue sliders and a hex field for one palette color.
fn color_picker(
    ui: &mut Ui<'_>,
    context: &mut Context<'_>,
    area: Rect,
    paint: Paint,
    original: Rgb,
) {
    let roles = ui.theme.roles;
    ui.canvas.fill(area, roles.panel_dark);
    let id = Id::new("picker");
    let hex = |color: Rgb| format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b);
    let mut color = context
        .fields
        .get(&id)
        .and_then(|text| Rgb::parse_hex(text).ok())
        .unwrap_or(original);
    let (preview, rest) = area.inset(2).take_left(ROW * 2);
    let mut rows = rest.inset(0);
    let mut row = || {
        let (taken, remaining) = rows.take_top(ROW + 1);
        rows = remaining;
        taken
    };
    for (channel, value) in [("r", color.r), ("g", color.g), ("b", color.b)] {
        let (caption, bar) = row().take_left(8);
        label(ui, caption, channel, roles.ink_dim);
        let mut number = f64::from(value);
        if slider(
            ui,
            id.with_name(channel),
            bar.inset(1),
            &mut number,
            (0.0, 255.0),
            1.0,
        ) {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "clamped to 0..=255"
            )]
            let byte = number.round().clamp(0.0, 255.0) as u8;
            match channel {
                "r" => color.r = byte,
                "g" => color.g = byte,
                _ => color.b = byte,
            }
        }
    }
    let field = id.with_name("hex");
    let mut text = context
        .fields
        .get(&field)
        .cloned()
        .unwrap_or_else(|| hex(color));
    let response = text_field(ui, field, row(), &mut text);
    if response.changed {
        if let Ok(parsed) = Rgb::parse_hex(&text) {
            color = parsed;
        }
        context.fields.insert(field, text);
    } else if !response.focused {
        context.fields.remove(&field);
    }
    context.fields.insert(id, hex(color));
    ui.canvas.rgb_box(
        rect(preview.x, preview.y, preview.width, preview.height / 2),
        original,
    );
    ui.canvas.rgb_box(
        rect(
            preview.x,
            preview.y + preview.height / 2,
            preview.width,
            preview.height / 2,
        ),
        color,
    );
    let (apply, cancel) = row().take_left(rest.width / 2);
    if button(ui, id.with_name("apply"), apply.inset(1), "Apply") {
        context.fields.remove(&id);
        context.intents.push(Intent::SetColor { paint, color });
        context.intents.push(Intent::ClosePicker);
    }
    if button(ui, id.with_name("cancel"), cancel.inset(1), "Cancel") {
        context.fields.remove(&id);
        context.intents.push(Intent::ClosePicker);
    }
}
