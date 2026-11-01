use px_ui::{Area, Id, Key, Ui, button, draw_text, panel, rect, scroll_area};

use super::Context;
use crate::app::Intent;

const HELP: [(&str, &[&str]); 9] = [
    (
        "Files and tabs",
        &[
            "File > New (Ctrl+N) and Open (Ctrl+O); recent files are listed in the File menu.",
            "Each open file is a tab under the menus; the home tab shows when none is active.",
            "Ctrl+Tab cycles tabs, Ctrl+Home shows the home tab, Ctrl+W closes a tab.",
            "Ctrl+S saves, Ctrl+Shift+S saves as. Export As: Ctrl+Alt+Shift+S; sprite sheet: Ctrl+E.",
        ],
    ),
    (
        "Painting",
        &[
            "Choose a pixel layer in Layers (shape layers can't be painted).",
            "Pick a tool on the right; the bar above says what it does.",
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
        "Sprite and elements",
        &[
            "Sprite > Canvas Size (C) resizes around an anchor; Properties (Ctrl+P) sets timing.",
            "Elements lists every node of the file; pick one to see its code and inspect it.",
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
            "The color bar on the left: click a swatch to paint; right click for the second.",
            "Palette > Edit palette (F4) adds ramps, generates tones, imports .hex or .gpl.",
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
            "Ctrl+Z undoes anything, from strokes to typing.",
        ],
    ),
];

const CLOSE_ROW: i32 = px_ui::ROW + 6;

pub(super) fn help(ui: &mut Ui<'_>, context: &mut Context<'_>) {
    let bounds = ui.canvas.bounds();
    let line = i32::from(ui.font.line_height()) + 1;
    let lines: i32 = HELP
        .iter()
        .map(|(_, text)| i32::try_from(text.len()).unwrap_or(0) + 1)
        .sum();
    let text_height = lines * line + 4;
    let width = (bounds.width - 40).min(470);
    let height = (text_height + CLOSE_ROW + px_ui::HEADER + 2).min(bounds.height - 20);
    let area = bounds.centered(width, height);
    ui.popup(area, |ui| {
        let roles = ui.theme.roles;
        let (_, body) = panel(ui, area, "How to use the editor");
        let (close_row, text_area) = body.take_bottom(CLOSE_ROW);
        let (view, offset) = scroll_area(
            ui,
            Id::new("help").with_name("scroll"),
            text_area,
            text_height,
        );
        let clip = ui.canvas.clip_to(view);
        let mut y = view.y + 2 - offset;
        for (topic, text) in HELP {
            draw_text(ui.canvas, ui.font, view.x + 4, y, topic, roles.accent_light);
            y += line;
            for sentence in text {
                draw_text(ui.canvas, ui.font, view.x + 10, y, sentence, roles.ink);
                y += line;
            }
        }
        ui.canvas.restore_clip(clip);
        let close = rect(close_row.right() - 54, close_row.y + 2, 50, px_ui::ROW + 2);
        if button(ui, Id::new("help").with_name("close"), close, "Close")
            || ui.input.key(Key::Escape)
        {
            context.intents.push(Intent::ToggleHelp);
        }
    });
}
