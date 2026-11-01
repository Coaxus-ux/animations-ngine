use px_ui::{
    Area, Id, Rect, TOOL_BUTTON, Ui, button, checkbox, icon_button, label, rect, text_width,
};

use super::{Context, Placement};
use crate::app::Intent;
use crate::domain::Document;
use crate::tools::ToolKind;

/// Height of the two paint boxes under the color bar (UI pixels).
pub(super) const PAINTS: i32 = 2 * TOOL_BUTTON;

/// The tools, one under another, down the window's right edge.
pub(super) fn column(ui: &mut Ui<'_>, context: &mut Context<'_>, area: Rect) {
    let roles = ui.theme.roles;
    ui.canvas.fill(area, roles.header);
    ui.canvas
        .fill(rect(area.x, area.y, 1, area.height), roles.panel_dark);
    let id = Id::new("tools");
    let x = area.x + 3;
    let mut y = area.y + 3;
    for tool in ToolKind::ALL {
        let slot = rect(x, y, TOOL_BUTTON, TOOL_BUTTON);
        let selected = context.editing.tool == tool;
        if icon_button(
            ui,
            id.with_name(tool.icon()),
            slot,
            tool.icon(),
            selected,
            tool.tip(),
        ) {
            context.intents.push(Intent::Tool(tool));
        }
        y += TOOL_BUTTON + 1;
    }
}

/// The main and second colors, one over the other as in Aseprite, and the swap button.
pub(super) fn paints(ui: &mut Ui<'_>, context: &mut Context<'_>, area: Rect) {
    let roles = ui.theme.roles;
    ui.canvas.fill(area, roles.panel);
    ui.canvas
        .fill(rect(area.x, area.y, area.width, 1), roles.panel_dark);
    let Some(palette) = context
        .document
        .and_then(Document::scene)
        .map(px_engine::Scene::palette)
    else {
        return;
    };
    let (x, y) = (area.x + 4, area.y + 4);
    let colors = [
        (
            context.editing.primary,
            rect(x, y, TOOL_BUTTON - 4, TOOL_BUTTON - 4),
            "Main color (left button)",
        ),
        (
            context.editing.secondary,
            rect(x + 4, y + TOOL_BUTTON - 4, TOOL_BUTTON - 4, TOOL_BUTTON - 4),
            "Second color (right button)",
        ),
    ];
    for (paint, at, tip) in colors.into_iter().rev() {
        ui.canvas.fill(at, roles.ink);
        if let Some(color) = paint.and_then(|paint| palette.resolve(paint).ok()) {
            ui.canvas.rgb_box(at.inset(1), color);
        } else {
            ui.canvas.fill(at.inset(1), roles.checker_light);
            ui.canvas.dither(at.inset(1), roles.checker_dark);
        }
        if ui.hovering(at) {
            ui.tooltip(tip);
        }
    }
    let swap = rect(
        x + TOOL_BUTTON + 6,
        y + TOOL_BUTTON / 2 - 2,
        TOOL_BUTTON,
        px_ui::ROW,
    );
    if button(ui, Id::new("tools").with_name("swap"), swap, "X") {
        context.intents.push(Intent::SwapPaints);
    }
}

/// The current tool in words, what the mouse does with it, and its options.
pub(super) fn options(ui: &mut Ui<'_>, context: &mut Context<'_>, area: Rect) {
    let roles = ui.theme.roles;
    ui.canvas.fill(area, roles.panel);
    ui.canvas.fill(
        rect(area.x, area.bottom() - 1, area.width, 1),
        roles.panel_dark,
    );
    let tool = context.editing.tool;
    let id = Id::new("tool-options");
    let row = area.inset(2);
    let name = tool.tip().split(" (").next().unwrap_or_default();
    let name_width = text_width(ui.font, name) + 8;
    label(
        ui,
        rect(row.x + 2, row.y, name_width, row.height),
        name,
        roles.accent_light,
    );
    let mut x = row.x + name_width + 4;
    let mut option = |ui: &mut Ui<'_>, text: &str, on: bool| {
        let width = text_width(ui.font, text) + 14;
        let clicked = checkbox(
            ui,
            id.with_name(text),
            rect(x, row.y, width, row.height),
            text,
            on,
        );
        x += width + 6;
        clicked
    };
    match tool {
        ToolKind::Rect | ToolKind::Ellipse => {
            if option(ui, "Filled", context.editing.filled) {
                context
                    .intents
                    .push(Intent::Filled(!context.editing.filled));
            }
        }
        ToolKind::Fill => {
            if option(ui, "Every matching pixel", context.editing.global_fill) {
                context
                    .intents
                    .push(Intent::GlobalFill(!context.editing.global_fill));
            }
        }
        ToolKind::Pick if context.editing.picked.is_some() => {
            let width = 110;
            if button(
                ui,
                id.with_name("recolor"),
                rect(x, row.y, width, row.height),
                "Use main color",
            ) {
                context.intents.push(Intent::RecolorPicked);
            }
            x += width + 6;
        }
        _ => {}
    }
    let (hint, ink) = match blocked(context, tool) {
        Some(problem) => (problem, roles.warning),
        None => (how_to(tool), roles.ink_dim),
    };
    label(
        ui,
        rect(x + 4, row.y, row.right() - x - 4, row.height),
        hint,
        ink,
    );
}

/// Why the tool can't be used on the active layer, if it can't.
fn blocked(context: &Context<'_>, tool: ToolKind) -> Option<&'static str> {
    if !tool.paints() {
        return None;
    }
    let layer = context
        .view
        .layer
        .and_then(|index| context.document?.layers().get(index));
    match layer {
        None => Some("Choose a pixel layer in Layers to paint on (or Layer > New pixel layer)"),
        Some(layer) if !layer.paintable() => {
            Some("This layer is made of shapes: choose a pixel layer, or Layer > New pixel layer")
        }
        Some(layer) if context.editing.locked.contains(&layer.name) => {
            Some("This layer is locked: unlock it in Layers")
        }
        Some(_) => None,
    }
}

const fn how_to(tool: ToolKind) -> &'static str {
    match tool {
        ToolKind::Pencil => "Left button paints with the main color, right with the second",
        ToolKind::Eraser => "Click or drag to clear pixels",
        ToolKind::Fill => "Click an area to fill it with the main color",
        ToolKind::Line => "Drag from one end to the other (Shift: straight angles)",
        ToolKind::Rect => "Drag from corner to corner (Shift: square)",
        ToolKind::Ellipse => "Drag from corner to corner (Shift: circle)",
        ToolKind::Eyedropper => "Click a pixel to take its color",
        ToolKind::Select => {
            "Drag a box to select; click to clear. Edit menu: cut, copy, paste, flip"
        }
        ToolKind::Lasso => "Trace around pixels to select them",
        ToolKind::Move => "Drag to move the selection (or the whole layer)",
        ToolKind::Pick => "Click a shape, then click a swatch in Palette to recolor it",
    }
}

/// Aseprite's status line: size, zoom, pointer, color, frame and layer; after a separator,
/// the file's first error, the last message, or what F1 does.
pub(super) fn status(
    ui: &mut Ui<'_>,
    context: &mut Context<'_>,
    area: Rect,
    canvas_area: Option<Rect>,
) {
    let roles = ui.theme.roles;
    ui.canvas.fill(area, roles.header);
    ui.canvas
        .fill(rect(area.x, area.y, area.width, 1), roles.panel_dark);
    let info = context
        .document
        .map(|document| facts(context, document, canvas_area).join(" | "))
        .unwrap_or_default();
    let info_width = (text_width(ui.font, &info) + 10).min(area.width * 2 / 3);
    let (left, right) = area.inset(1).take_left(info_width);
    label(ui, left, &format!(" {info}"), roles.ink);
    ui.canvas
        .fill(rect(left.right(), left.y, 1, left.height), roles.panel_dark);
    let (message, ink) = match context
        .document
        .and_then(|document| document.errors().first())
    {
        Some(error) => (error.clone(), roles.warning),
        None if context.editing.status.is_empty() => {
            ("F1: help and shortcuts".to_owned(), roles.ink_dim)
        }
        None => (context.editing.status.clone(), roles.ink_dim),
    };
    label(
        ui,
        rect(right.x + 4, right.y, right.width - 4, right.height),
        &message,
        ink,
    );
}

fn facts(context: &Context<'_>, document: &Document, canvas_area: Option<Rect>) -> Vec<String> {
    let mut parts = Vec::new();
    let Some(scene) = document.scene() else {
        return parts;
    };
    let size = (
        i32::from(scene.size().width()),
        i32::from(scene.size().height()),
    );
    parts.push(format!("{}x{}", size.0, size.1));
    if let Some(area) = canvas_area {
        let scale = context.scale;
        let window = rect(
            area.x * scale,
            area.y * scale,
            area.width * scale,
            area.height * scale,
        );
        let zoom = Placement::of(context.view, window, size).zoom;
        parts.push(format!("zoom {}%", zoom * 100));
    }
    if let Some((x, y)) = context.hover {
        parts.push(format!("{x},{y}"));
    }
    if let (Some(paint), Some(loaded)) = (context.editing.primary, document.loaded()) {
        parts.push(format!(
            "color {}",
            loaded.paint_names.paint(paint).unwrap_or_default()
        ));
    }
    let frames = scene.timing().duration.get();
    parts.push(format!(
        "frame {}/{frames}",
        context.view.frame % frames.max(1) + 1
    ));
    if let Some(layer) = context
        .view
        .layer
        .and_then(|index| document.layers().get(index))
    {
        parts.push(format!("layer {}", layer.name));
    }
    parts
}
