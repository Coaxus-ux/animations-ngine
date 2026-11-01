use px_core::MAX_TONES;
use px_ui::{Area, Id, ROW, Rect, Ui, icon_button, label, rect, text_field};

use super::{Column, id, select};
use crate::app::Intent;
use crate::domain::palette_model::{PaletteModel, RampModel};
use crate::views::Context;

const ROW_HEIGHT: i32 = ROW + 1;
const NAME: i32 = 30;
const BUTTONS: i32 = 3 * ROW;
const SWATCH: (i32, i32) = (4, 9);

pub(super) fn height(model: &PaletteModel) -> i32 {
    i32::try_from(model.ramps.len()).unwrap_or(0) * ROW_HEIGHT
}

pub(super) fn rows(
    ui: &mut Ui<'_>,
    context: &mut Context<'_>,
    model: &PaletteModel,
    selected: Option<(usize, usize)>,
    column: &mut Column,
) {
    for (index, ramp) in model.ramps.iter().enumerate() {
        let row = column.row(ROW_HEIGHT);
        if !column.shows(row) {
            continue;
        }
        let row_id = id().with_name("ramp").with(index as u64);
        let (name, rest) = row.take_left(NAME);
        let (buttons, swatches) = rest.take_right(BUTTONS);
        name_cell(ui, context, row_id, ramp, name);
        let selected_tone = selected.and_then(|(at, tone)| (at == index).then_some(tone));
        tones(ui, context, row_id, ramp, selected_tone, swatches);
        ramp_buttons(ui, context, row_id, ramp, model.ramps.len(), buttons);
    }
}

fn name_cell(ui: &mut Ui<'_>, context: &mut Context<'_>, row_id: Id, ramp: &RampModel, cell: Rect) {
    let roles = ui.theme.roles;
    let renaming = id().with_name("renaming");
    let field = id().with_name("rename-text");
    if context.fields.get(&renaming) == Some(&ramp.name) {
        let text = context
            .fields
            .entry(field)
            .or_insert_with(|| ramp.name.clone());
        let response = text_field(ui, field, cell, text);
        if response.submitted {
            let name = context.fields.remove(&field).unwrap_or_default();
            context.fields.remove(&renaming);
            if !name.is_empty() && name != ramp.name {
                context.intents.push(Intent::SetRamp {
                    ramp: ramp.name.clone(),
                    name,
                    colors: ramp.colors.clone(),
                });
            }
        } else if !response.focused {
            context.fields.remove(&field);
            context.fields.remove(&renaming);
        }
        return;
    }
    let response = ui.interact(row_id.with_name("name"), cell);
    if response.hovered {
        ui.canvas.dither(cell, roles.panel_light);
        ui.tooltip("Double click to rename");
    }
    label(ui, cell, &ramp.name, roles.ink_dim);
    if response.double_clicked {
        context.fields.insert(renaming, ramp.name.clone());
        context.fields.remove(&field);
        ui.state.focus = Some(field);
    } else if response.clicked {
        select(context, &ramp.name, 0);
    }
}

fn tones(
    ui: &mut Ui<'_>,
    context: &mut Context<'_>,
    row_id: Id,
    ramp: &RampModel,
    selected: Option<usize>,
    area: Rect,
) {
    let roles = ui.theme.roles;
    let tones = i32::try_from(MAX_TONES).unwrap_or(8);
    let side = (area.width / tones - 1).clamp(SWATCH.0, SWATCH.1);
    for (tone, color) in ramp.colors.iter().enumerate() {
        let x = area.x + i32::try_from(tone).unwrap_or(0) * (side + 1);
        let cell = rect(x, area.y + 1, side, side.min(area.height - 2));
        let response = ui.interact(row_id.with_name("tone").with(tone as u64), cell);
        ui.canvas.rgb_box(cell, *color);
        let outline = rect(cell.x - 1, cell.y - 1, cell.width + 2, cell.height + 2);
        if selected == Some(tone) {
            ui.canvas.frame(outline, roles.accent_light);
        } else if response.hovered {
            ui.canvas.frame(outline, roles.ink);
        }
        if response.clicked {
            select(context, &ramp.name, tone);
        }
    }
}

fn ramp_buttons(
    ui: &mut Ui<'_>,
    context: &mut Context<'_>,
    row_id: Id,
    ramp: &RampModel,
    ramp_count: usize,
    area: Rect,
) {
    let (plus, rest) = area.take_left(ROW);
    let (minus, trash) = rest.take_left(ROW);
    let square = |cell: Rect| rect(cell.x, cell.y, ROW, ROW);
    let count = ramp.colors.len();
    if count < MAX_TONES
        && icon_button(
            ui,
            row_id.with_name("plus"),
            square(plus),
            "plus",
            false,
            "Add a tone",
        )
    {
        let mut colors = ramp.colors.clone();
        colors.extend(ramp.colors.last().copied());
        context.intents.push(Intent::SetRamp {
            ramp: ramp.name.clone(),
            name: ramp.name.clone(),
            colors,
        });
    }
    if count > 1
        && icon_button(
            ui,
            row_id.with_name("minus"),
            square(minus),
            "minus",
            false,
            "Drop the last tone",
        )
    {
        let mut colors = ramp.colors.clone();
        colors.pop();
        context.intents.push(Intent::SetRamp {
            ramp: ramp.name.clone(),
            name: ramp.name.clone(),
            colors,
        });
    }
    if ramp_count > 1
        && icon_button(
            ui,
            row_id.with_name("trash"),
            square(trash),
            "trash",
            false,
            "Remove the ramp (refused while something paints with it)",
        )
    {
        context.intents.push(Intent::RemoveRamp(ramp.name.clone()));
    }
}
