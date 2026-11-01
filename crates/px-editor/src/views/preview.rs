use px_ui::{Area, ROW, Rect, Ui, label, pack, rect};

use super::{Context, RenderCache};
use crate::app::{CodeMode, PanelKind, WindowKind};

pub(super) fn strip(ui: &mut Ui<'_>, context: &mut Context<'_>, area: Rect) -> Rect {
    let wanted = match context.kind {
        WindowKind::Panel(kind) => kind == PanelKind::Code,
        WindowKind::Main => context.view.code == CodeMode::Only,
        WindowKind::Canvas => false,
    };
    let Some((width, height, _, _)) = context.cache.image().filter(|_| wanted) else {
        return area;
    };
    let scale = context.scale.max(1);
    let room = (
        (area.width - 2) * scale,
        (area.height / 3 - ROW).max(ROW) * scale,
    );
    let (_, shown_height) = fit((width, height), room);
    let (strip, rest) = area.take_bottom(ROW + 2 + (shown_height + scale - 1) / scale);
    let roles = ui.theme.roles;
    ui.canvas.fill(strip, roles.panel_dark);
    let (title, body) = strip.take_top(ROW);
    label(ui, title, " Preview", roles.ink_dim);
    context.preview = Some(body.inset(1));
    rest
}

pub fn paint_preview(
    pixels: &mut [u32],
    stride: usize,
    area: Rect,
    scale: i32,
    cache: &RenderCache,
) {
    let Some((width, height, cells, palette)) = cache.image() else {
        return;
    };
    let clip = rect(
        area.x * scale,
        area.y * scale,
        area.width * scale,
        area.height * scale,
    );
    let (shown_width, shown_height) = fit((width, height), (clip.width, clip.height));
    let left = clip.x + (clip.width - shown_width) / 2;
    let top = clip.y + (clip.height - shown_height) / 2;
    for y in 0..shown_height {
        let source_row = y * height / shown_height.max(1) * width;
        let Ok(row) = usize::try_from(top + y) else {
            continue;
        };
        for x in 0..shown_width {
            let source = usize::try_from(source_row + x * width / shown_width.max(1)).unwrap_or(0);
            let (Ok(column), Some(paint)) = (usize::try_from(left + x), cells.get(source)) else {
                continue;
            };
            if column < stride
                && let Some(pixel) = pixels.get_mut(row * stride + column)
            {
                *pixel = pack(palette.resolve(*paint).unwrap_or_default());
            }
        }
    }
}

fn fit((width, height): (i32, i32), (room_width, room_height): (i32, i32)) -> (i32, i32) {
    let (width, height) = (width.max(1), height.max(1));
    let zoom = (room_width / width).min(room_height / height);
    if zoom >= 1 {
        return (width * zoom, height * zoom);
    }
    if room_width * height <= room_height * width {
        (room_width.max(1), (room_width * height / width).max(1))
    } else {
        ((room_height * width / height).max(1), room_height.max(1))
    }
}
