use px_ui::{Area, Id, Key, Ui, button, draw_text, panel, rect};

use super::Context;
use crate::app::Intent;

const HELP: [(&str, &[&str]); 7] = [
    (
        "Painting",
        &[
            "Choose a pixel layer in Layers (shape layers can't be painted).",
            "Pick a tool on the left; the bar above says what it does.",
            "Left button: main color. Right button: second color. X swaps them.",
            "Wheel or + and - zoom; Space+drag or the middle button pans; 0 fits.",
        ],
    ),
    (
        "Selecting",
        &[
            "Select box (M) or lasso (Q), then Edit: cut, copy, paste, flip, delete.",
            "Move (V) drags the selection, or the whole layer without one.",
        ],
    ),
    (
        "Layers",
        &[
            "Eye: show or hide in this window. Lock: stop painting it.",
            "Drag a layer to reorder it; the buttons below add, rename, delete, move.",
        ],
    ),
    (
        "Animation",
        &[
            "Each layer's cel shows from its frame until the next cel (Timeline).",
            "New cel copies what shows at this frame. Enter plays; , and . step.",
        ],
    ),
    (
        "Colors",
        &[
            "Click a swatch to paint with it; right click for the second color.",
            "Edit color changes the palette itself: everything using it changes.",
            "Pick tool (O): click a shape, then a swatch to recolor that shape.",
        ],
    ),
    (
        "Code",
        &[
            "F2 shows the scene's KDL beside the canvas. Type; it loads after a pause.",
            "Every change you make anywhere is written in that text: it is the file.",
        ],
    ),
    (
        "Windows",
        &[
            "Window menu: put any panel in its own window, or open another canvas.",
            "Ctrl+Z undoes anything, from strokes to typing. Ctrl+S saves.",
        ],
    ),
];

/// The help screen, over everything; F1, Escape or Close hides it.
pub(super) fn help(ui: &mut Ui<'_>, context: &mut Context<'_>) {
    let bounds = ui.canvas.bounds();
    let line = i32::from(ui.font.line_height()) + 1;
    let lines: i32 = HELP
        .iter()
        .map(|(_, text)| i32::try_from(text.len()).unwrap_or(0) + 1)
        .sum();
    let width = (bounds.width - 40).min(420);
    let height = (lines * line + 40).min(bounds.height - 20);
    let area = bounds.centered(width, height);
    ui.popup(area, |ui| {
        let roles = ui.theme.roles;
        let (_, body) = panel(ui, area, "How to use the editor");
        let mut y = body.y + 2;
        for (topic, text) in HELP {
            draw_text(ui.canvas, ui.font, body.x + 4, y, topic, roles.accent_light);
            y += line;
            for sentence in text {
                draw_text(ui.canvas, ui.font, body.x + 10, y, sentence, roles.ink);
                y += line;
            }
        }
        let close = rect(
            body.right() - 54,
            body.bottom() - px_ui::ROW - 3,
            50,
            px_ui::ROW + 2,
        );
        if button(ui, Id::new("help").with_name("close"), close, "Close")
            || ui.input.key(Key::Escape)
        {
            context.intents.push(Intent::ToggleHelp);
        }
    });
}
