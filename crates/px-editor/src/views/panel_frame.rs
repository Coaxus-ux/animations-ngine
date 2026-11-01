use px_ui::{Area, HEADER, Id, Rect, Ui, header_button, panel, rect};

use super::{Context, code, elements, inspector, layers, palette, palette_edit, timeline};
use crate::app::{Intent, PanelKind, WindowKind};

pub(super) const COLLAPSED: i32 = HEADER + 2;

pub(super) fn collapsible(context: &Context<'_>, kind: PanelKind) -> bool {
    context.kind == WindowKind::Main && kind != PanelKind::Code
}

pub(super) fn is_collapsed(context: &Context<'_>, kind: PanelKind) -> bool {
    collapsible(context, kind) && context.panels.is_collapsed(kind)
}

pub(super) fn panel_view(ui: &mut Ui<'_>, context: &mut Context<'_>, kind: PanelKind, area: Rect) {
    let (header, body) = panel(ui, area, kind.title());
    let id = Id::new("panel").with_name(kind.title());
    let torn = matches!(context.kind, WindowKind::Panel(_));
    let mut slots = own_buttons(ui, context, kind, id, header);
    if collapsible(context, kind) {
        let (icon, tip) = if context.panels.is_collapsed(kind) {
            ("plus", "Expand")
        } else {
            ("minus", "Collapse (or click the title)")
        };
        if header_button(ui, id.with_name("fold"), header, slots, icon, tip) {
            context.panels.toggle_collapsed(kind);
        }
        slots += 1;
    }
    let tip = if torn {
        "Dock back into the main window"
    } else {
        "Open in its own window (or drag the title out)"
    };
    if header_button(ui, id.with_name("pop"), header, slots, "popout", tip) {
        context.intents.push(if torn {
            Intent::Dock(kind)
        } else {
            Intent::PopOut(kind)
        });
    }
    let title = rect(
        header.x,
        header.y,
        header.width - HEADER * (slots + 1),
        header.height,
    );
    let response = ui.interact(id.with_name("title"), title);
    let (px, py) = ui.input.pointer_anywhere;
    if !torn && response.released && !ui.canvas.bounds().contains(px, py) {
        context.intents.push(Intent::PopOut(kind));
    } else if response.clicked && collapsible(context, kind) {
        context.panels.toggle_collapsed(kind);
    }
    if is_collapsed(context, kind) {
        return;
    }
    match kind {
        PanelKind::Layers => layers::layers(ui, context, body),
        PanelKind::Palette if context.palette_edit => palette_edit::palette_edit(ui, context, body),
        PanelKind::Palette => palette::palette(ui, context, body),
        PanelKind::Elements => elements::elements(ui, context, body),
        PanelKind::Timeline => timeline::timeline(ui, context, body),
        PanelKind::Inspector => inspector::inspector(ui, context, body),
        PanelKind::Code => code::code(ui, context, body),
    }
}

fn own_buttons(
    ui: &mut Ui<'_>,
    context: &mut Context<'_>,
    kind: PanelKind,
    id: Id,
    header: Rect,
) -> i32 {
    let docked = context.kind == WindowKind::Main;
    match kind {
        PanelKind::Code if docked => code::pane_buttons(ui, context, id, header),
        PanelKind::Palette if docked => {
            let tip = "Move the palette to the other column";
            if header_button(ui, id.with_name("side"), header, 0, "side", tip) {
                context.intents.push(Intent::TogglePaletteSide);
            }
            1
        }
        PanelKind::Elements => {
            let (icon, tip) = if context.panels.every_element {
                (
                    "scene",
                    "Showing every element: show the current layer only",
                )
            } else {
                ("filter", "Showing the current layer: show every element")
            };
            if header_button(ui, id.with_name("scope"), header, 0, icon, tip) {
                context.panels.every_element = !context.panels.every_element;
            }
            1
        }
        _ => 0,
    }
}
