use px_ui::{Area, Id, ROW, Rect, Ui, button, checkbox, label, rect, text_width};

use super::Context;
use crate::app::Intent;

pub(super) const BUTTON: i32 = ROW + 1;

pub(super) fn controls(
    ui: &mut Ui<'_>,
    context: &mut Context<'_>,
    bar: Rect,
    frame: u32,
    frames: u32,
) {
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
