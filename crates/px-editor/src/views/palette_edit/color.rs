use px_core::{Hsv, MAX_TONES, Paint, Rgb, Tone};
use px_ui::{Area, Rect, Ui, button, hue_bar, label, number_field, sv_square, text_field};

use super::{BUTTON_ROW, Column, GAP, id};
use crate::app::Intent;
use crate::domain::palette_model::PaletteModel;
use crate::views::Context;

const SQUARE: i32 = 32;
const HUE: i32 = 8;
const CAPTION: i32 = 22;
pub(super) const HEIGHT: i32 = GAP + SQUARE + 1 + HUE + 1 + BUTTON_ROW * 4;

struct Setting {
    key: &'static str,
    caption: &'static str,
    tip: &'static str,
    default: i64,
    range: (i64, i64),
}

const SETTINGS: [Setting; 4] = [
    Setting {
        key: "steps",
        caption: "Len",
        tip: "How many tones the ramp gets",
        default: 4,
        range: (2, 8),
    },
    Setting {
        key: "hue",
        caption: "Hue",
        tip: "Degrees the hue turns from dark to light tones",
        default: 15,
        range: (-60, 60),
    },
    Setting {
        key: "contrast",
        caption: "Con",
        tip: "How far the tones spread from dark to light, in percent",
        default: 40,
        range: (0, 100),
    },
    Setting {
        key: "saturation",
        caption: "Sat",
        tip: "How much the light tones desaturate, in percent",
        default: 20,
        range: (0, 100),
    },
];

pub(super) fn editor(
    ui: &mut Ui<'_>,
    context: &mut Context<'_>,
    model: &PaletteModel,
    (ramp, tone): (usize, usize),
    column: &mut Column,
) {
    let Some(ramp) = model.ramps.get(ramp) else {
        return;
    };
    let Some(current) = ramp.colors.get(tone).copied() else {
        return;
    };
    let paint = context
        .document
        .and_then(|document| document.loaded())
        .and_then(|loaded| loaded.paint_names.find(&ramp.name))
        .map(|id| Paint {
            ramp: id,
            tone: Tone::new(u8::try_from(tone).unwrap_or(u8::MAX)),
        });
    let Some(paint) = paint else {
        return;
    };
    column.row(GAP);
    let square = column.row(SQUARE);
    column.row(1);
    let hue = column.row(HUE);
    column.row(1);
    let hex_row = column.row(BUTTON_ROW);
    if column.shows(square) || column.shows(hex_row) {
        picker(ui, context, paint, current, (square, hue, hex_row));
    }
    generator(ui, context, &ramp.name, current, ramp.colors.len(), column);
}

fn picker(
    ui: &mut Ui<'_>,
    context: &mut Context<'_>,
    paint: Paint,
    current: Rgb,
    (square, hue, hex_row): (Rect, Rect, Rect),
) {
    let roles = ui.theme.roles;
    let hsv_id = id().with_name("hsv");
    let mut hsv = context
        .fields
        .get(&hsv_id)
        .and_then(|text| parse_hsv(text))
        .filter(|hsv| hsv.to_rgb() == current)
        .unwrap_or_else(|| Hsv::from_rgb(current));
    let changed = sv_square(ui, id().with_name("sv"), square.inset(1), &mut hsv)
        | hue_bar(ui, id().with_name("hue"), hue.inset(1), &mut hsv);
    if changed {
        let color = hsv.to_rgb();
        context
            .fields
            .insert(hsv_id, format!("{} {} {}", hsv.h, hsv.s, hsv.v));
        context.intents.push(Intent::SetColor { paint, color });
    }
    let (caption, field) = hex_row.inset(1).take_left(CAPTION);
    label(ui, caption, "Hex", roles.ink_dim);
    let hex_id = id().with_name("hex");
    let mut text = context
        .fields
        .get(&hex_id)
        .cloned()
        .unwrap_or_else(|| current.to_string());
    let response = text_field(ui, hex_id, field, &mut text);
    if response.changed {
        if let Ok(color) = Rgb::parse_hex(&text) {
            context.intents.push(Intent::SetColor { paint, color });
        }
        context.fields.insert(hex_id, text);
    } else if !response.focused {
        context.fields.remove(&hex_id);
    }
}

fn generator(
    ui: &mut Ui<'_>,
    context: &mut Context<'_>,
    ramp: &str,
    base: Rgb,
    tones: usize,
    column: &mut Column,
) {
    let roles = ui.theme.roles;
    let mut values = [0i64; 4];
    for pair in [(0usize, 1usize), (2, 3)] {
        let row = column.row(BUTTON_ROW);
        let (left, right) = row.inset(1).take_left(row.width / 2);
        for (slot, cell) in [pair.0, pair.1].into_iter().zip([left, right]) {
            let Setting {
                key,
                caption,
                tip,
                mut default,
                mut range,
            } = SETTINGS[slot];
            if key == "steps" {
                range.1 = i64::try_from(MAX_TONES).unwrap_or(range.1);
                default = i64::try_from(tones)
                    .unwrap_or(default)
                    .clamp(range.0, range.1);
            }
            let field = id().with_name(key);
            let mut value = context
                .fields
                .get(&field)
                .and_then(|text| text.parse().ok())
                .unwrap_or(default);
            if column.shows(row) {
                let (name, number) = cell.take_left(CAPTION);
                label(ui, name, caption, roles.ink_dim);
                if ui.hovering(name) {
                    ui.tooltip(tip);
                }
                if number_field(ui, field, number, &mut value, range) {
                    context.fields.insert(field, value.to_string());
                }
            }
            values[slot] = value;
        }
    }
    let row = column.row(BUTTON_ROW);
    if column.shows(row)
        && button(
            ui,
            id().with_name("generate"),
            row.inset(1),
            "Generate from selected",
        )
    {
        let [steps, hue_shift, contrast, saturation] = values;
        context.intents.push(Intent::GenerateRamp {
            ramp: ramp.to_owned(),
            base,
            steps: u8::try_from(steps).unwrap_or(1),
            hue_shift: whole(hue_shift),
            contrast: whole(contrast) / 100.0,
            saturation: whole(saturation) / 100.0,
        });
    }
    if ui.hovering(row) {
        ui.tooltip("Rebuild the ramp's tones from the selected color, dark to light");
    }
}

fn whole(value: i64) -> f64 {
    f64::from(i32::try_from(value).unwrap_or(0))
}

fn parse_hsv(text: &str) -> Option<Hsv> {
    let mut parts = text.split(' ').map(str::parse::<f64>);
    let (h, s, v) = (
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    );
    Some(Hsv::new(h, s, v))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hsv_draft_round_trips_through_its_text() {
        let hsv = Hsv::new(200.5, 0.25, 0.75);
        let text = format!("{} {} {}", hsv.h, hsv.s, hsv.v);
        assert_eq!(parse_hsv(&text), Some(hsv));
        assert_eq!(parse_hsv("1 2"), None);
    }
}
