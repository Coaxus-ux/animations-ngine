use px_engine::Geometry;
use px_ui::{Area, Id, Input, Key, Rect, Theme, Ui, WHEEL_NOTCH, pack, rect};

use super::doc_view::{
    Placement, RenderCache, ViewPixels, draw_cursor, draw_document, draw_grid, draw_selection,
};
use super::{Context, unavailable};
use crate::app::{Button, Editing, Intent, ViewState};
use crate::domain::Document;

pub(super) fn canvas(ui: &mut Ui<'_>, context: &mut Context<'_>, area: Rect) -> Option<Rect> {
    let roles = ui.theme.roles;
    let Some(scene) = context.document.and_then(Document::scene) else {
        ui.canvas.fill(area, roles.window);
        unavailable(ui, context, area);
        return None;
    };
    let scale = context.scale;
    let window = rect(
        area.x * scale,
        area.y * scale,
        area.width * scale,
        area.height * scale,
    );
    let size = (
        i32::from(scene.size().width()),
        i32::from(scene.size().height()),
    );
    let placement = Placement::of(context.view, window, size);
    let raw = context.raw;
    let response = ui.interact(Id::new("canvas"), area);
    let to_canvas = |(x, y): (i32, i32)| placement.to_canvas(x, y);
    context.hover = raw
        .pointer
        .map(to_canvas)
        .filter(|(x, y)| *x >= 0 && *y >= 0 && *x < size.0 && *y < size.1);
    let previous = context.last_pointer.replace(raw.pointer_anywhere);
    let panning = raw.holding(Key::Space) || raw.down.middle;
    if panning
        && (raw.down.left || raw.down.middle)
        && let Some((px, py)) = previous
    {
        let (dx, dy) = (raw.pointer_anywhere.0 - px, raw.pointer_anywhere.1 - py);
        if (dx, dy) != (0, 0) {
            context.intents.push(Intent::Pan { dx, dy });
        }
        return Some(area);
    }
    let steps = if response.hovered {
        -ui.wheel_steps(Id::new("canvas"), WHEEL_NOTCH).1
    } else {
        0
    };
    if steps != 0 {
        let at = raw.pointer.unwrap_or_default();
        context.intents.push(Intent::Zoom { steps, at });
    }
    let at = to_canvas(raw.pointer_anywhere);
    if response.pressed {
        context.intents.push(Intent::CanvasPress {
            at,
            button: Button::Primary,
        });
    } else if response.hovered && raw.pressed.right {
        context.intents.push(Intent::CanvasPress {
            at,
            button: Button::Secondary,
        });
    } else if response.released || raw.released.right {
        context.intents.push(Intent::CanvasRelease { at });
    } else if response.held || raw.down.right {
        context.intents.push(Intent::CanvasDrag { at });
    }
    Some(area)
}

pub struct CanvasPaint<'a> {
    pub document: Option<&'a Document>,
    pub view: &'a ViewState,
    pub cache: &'a RenderCache,
    pub editing: &'a Editing,
    pub guide: &'a [(i32, i32)],
    pub input: &'a Input,
    pub theme: &'a Theme,
}

pub fn paint_canvas(
    pixels: &mut [u32],
    width: usize,
    area: Rect,
    scale: i32,
    paint: &CanvasPaint<'_>,
) {
    let clip = rect(
        area.x * scale,
        area.y * scale,
        area.width * scale,
        area.height * scale,
    );
    let Some((canvas_width, canvas_height, _, _)) = paint.cache.image() else {
        return;
    };
    let canvas = (canvas_width, canvas_height);
    let placement = Placement::of(paint.view, clip, canvas);
    let palette = &paint.theme.palette;
    let roles = paint.theme.roles;
    let color = |role| pack(palette.resolve(role).unwrap_or_default());
    let mut target = ViewPixels {
        out: pixels,
        width,
        clip,
    };
    draw_document(&mut target, paint.cache, placement, (palette, &roles));
    if paint.view.onion
        && let Some(document) = paint.document
    {
        onion(
            &mut target,
            placement,
            document,
            paint.view,
            (color(roles.accent), color(roles.accent_light)),
        );
    }
    if paint.view.grid {
        draw_grid(&mut target, placement, canvas, color(roles.grid));
    }
    draw_selection(
        &mut target,
        placement,
        &paint.editing.selection,
        (color(roles.ink), color(roles.field)),
        paint.input.time_ms,
    );
    for point in paint.guide {
        target.block(placement, *point, color(roles.accent_light), true);
    }
    if let Some((x, y)) = paint.input.pointer
        && clip.contains(x, y)
    {
        draw_cursor(
            &mut target,
            placement,
            placement.to_canvas(x, y),
            color(roles.ink),
        );
    }
}

fn onion(
    target: &mut ViewPixels<'_>,
    placement: Placement,
    document: &Document,
    view: &ViewState,
    colors: (u32, u32),
) {
    let Some(scene) = document.scene() else {
        return;
    };
    let Some(art) = view
        .layer
        .and_then(|index| document.layers().get(index))
        .and_then(|layer| layer.art.as_ref())
    else {
        return;
    };
    let (Some(origin), Some(Geometry::Pixels(pixels))) = (
        art.origin,
        scene.shapes().get(art.shape).map(|shape| &shape.geometry),
    ) else {
        return;
    };
    let showing = pixels.cel_at(view.frame).map(|cel| cel.frame);
    let before = view
        .frame
        .checked_sub(1)
        .and_then(|frame| pixels.cel_at(frame));
    let after = pixels.cels.iter().find(|cel| Some(cel.frame) > showing);
    for (cel, color) in [(before, colors.0), (after, colors.1)] {
        let Some(cel) = cel.filter(|cel| Some(cel.frame) != showing) else {
            continue;
        };
        let columns = usize::from(cel.width.max(1));
        for (row, paints) in (0i32..).zip(cel.pixels.chunks(columns)) {
            for (column, paint) in (0i32..).zip(paints) {
                if paint.is_some() {
                    let at = (origin.0 + cel.x + column, origin.1 + cel.y + row);
                    target.block(placement, at, color, true);
                }
            }
        }
    }
}
