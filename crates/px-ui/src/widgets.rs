use px_core::Paint;

use crate::area::{Area, Rect, rect};
use crate::text::{draw_text, fit_text, text_width};
use crate::ui::{Id, Ui};

pub const ROW: i32 = 11;
pub const HEADER: i32 = 11;
pub const ICON: i32 = 16;
pub const TOOL_BUTTON: i32 = ICON + 4;
pub const SMALL_ICON: i32 = 8;

pub fn label(ui: &mut Ui<'_>, area: Rect, text: &str, paint: Paint) {
    let fitted = fit_text(ui.font, text, area.width);
    let y = area.y + (area.height - i32::from(ui.font.height())) / 2;
    draw_text(ui.canvas, ui.font, area.x, y, &fitted, paint);
}

pub fn centered_label(ui: &mut Ui<'_>, area: Rect, text: &str, paint: Paint) {
    let fitted = fit_text(ui.font, text, area.width);
    let width = text_width(ui.font, &fitted);
    let y = area.y + (area.height - i32::from(ui.font.height())) / 2;
    draw_text(
        ui.canvas,
        ui.font,
        area.x + (area.width - width) / 2,
        y,
        &fitted,
        paint,
    );
}

fn button_face(ui: &mut Ui<'_>, area: Rect, hovered: bool, sunken: bool, selected: bool) {
    let roles = ui.theme.roles;
    let face = if selected { roles.accent } else { roles.panel };
    ui.canvas.fill(area, face);
    if hovered && !selected {
        ui.canvas.dither(area.inset(1), roles.panel_light);
    }
    ui.canvas.bevel(
        area,
        roles.panel_light,
        roles.panel_dark,
        sunken || selected,
    );
}

pub fn button(ui: &mut Ui<'_>, id: Id, area: Rect, text: &str) -> bool {
    let response = ui.interact(id, area);
    button_face(ui, area, response.hovered, response.held, false);
    let offset = i32::from(response.held);
    let ink = ui.theme.roles.ink;
    centered_label(
        ui,
        rect(area.x + offset, area.y + offset, area.width, area.height),
        text,
        ink,
    );
    response.clicked
}

pub fn icon_button(
    ui: &mut Ui<'_>,
    id: Id,
    area: Rect,
    icon: &str,
    selected: bool,
    tip: &str,
) -> bool {
    let response = ui.interact(id, area);
    button_face(ui, area, response.hovered, response.held, selected);
    let offset = i32::from(response.held || selected);
    draw_icon(ui, area, icon, offset, None);
    if response.hovered && !tip.is_empty() {
        ui.tooltip(tip);
    }
    response.clicked
}

pub fn toggle_icon(
    ui: &mut Ui<'_>,
    id: Id,
    area: Rect,
    on: bool,
    icons: (&str, &str),
    tip: &str,
) -> bool {
    let response = ui.interact(id, area);
    if response.hovered {
        ui.canvas.dither(area, ui.theme.roles.panel_light);
        if !tip.is_empty() {
            ui.tooltip(tip);
        }
    }
    let tint = (!on).then_some(ui.theme.roles.ink_dim);
    draw_icon(ui, area, if on { icons.0 } else { icons.1 }, 0, tint);
    response.clicked
}

fn draw_icon(ui: &mut Ui<'_>, area: Rect, icon: &str, offset: i32, tint: Option<Paint>) {
    if let Some(bitmap) = ui.icons.get(icon) {
        let x = area.x + (area.width - bitmap.width()) / 2 + offset;
        let y = area.y + (area.height - bitmap.height()) / 2 + offset;
        ui.canvas.blit(bitmap, x, y, tint);
    } else {
        let letter: String = icon.chars().take(1).collect::<String>().to_uppercase();
        let ink = tint.unwrap_or(ui.theme.roles.ink);
        centered_label(ui, area, &letter, ink);
    }
}

pub fn checkbox(ui: &mut Ui<'_>, id: Id, area: Rect, text: &str, checked: bool) -> bool {
    let roles = ui.theme.roles;
    let response = ui.interact(id, area);
    if response.hovered {
        ui.canvas.dither(area, roles.panel_light);
    }
    let side = 7;
    let mark = rect(area.x + 1, area.y + (area.height - side) / 2, side, side);
    ui.canvas.fill(mark, roles.field);
    ui.canvas.frame(mark, roles.ink_dim);
    if checked {
        ui.canvas.fill(mark.inset(2), roles.accent_light);
    }
    let rest = rect(mark.right() + 3, area.y, area.width - side - 4, area.height);
    label(ui, rest, text, roles.ink);
    response.clicked
}

pub fn panel(ui: &mut Ui<'_>, area: Rect, title: &str) -> (Rect, Rect) {
    let roles = ui.theme.roles;
    ui.canvas.fill(area, roles.panel);
    ui.canvas.frame(area, roles.panel_dark);
    let (header, body) = area.inset(1).take_top(HEADER);
    ui.canvas.fill(header, roles.header);
    ui.canvas.fill(
        rect(header.x, header.bottom() - 1, header.width, 1),
        roles.panel_dark,
    );
    label(
        ui,
        header.inset(1).take_left(header.width - 2).0.inset(0),
        &format!(" {title}"),
        roles.ink,
    );
    (header, body.inset(1))
}

pub fn header_button(
    ui: &mut Ui<'_>,
    id: Id,
    header: Rect,
    slot: i32,
    icon: &str,
    tip: &str,
) -> bool {
    let side = header.height;
    let area = rect(header.right() - side * (slot + 1), header.y, side, side);
    toggle_icon(ui, id, area, true, (icon, icon), tip)
}

pub fn separator(ui: &mut Ui<'_>, area: Rect) {
    let y = area.y + area.height / 2;
    ui.canvas
        .fill(rect(area.x, y, area.width, 1), ui.theme.roles.panel_dark);
}

pub(crate) fn draw_tooltip(ui: &mut Ui<'_>, text: &str, x: i32, y: i32) {
    let roles = ui.theme.roles;
    let width = text_width(ui.font, text) + 6;
    let height = i32::from(ui.font.height()) + 6;
    let bounds = ui.canvas.bounds();
    let left = (x + 8).min(bounds.right() - width).max(0);
    let top = if y + 12 + height > bounds.bottom() {
        y - height - 2
    } else {
        y + 12
    };
    let area = rect(left, top, width, height);
    let clip = ui.canvas.clip_to(bounds);
    ui.canvas.fill(area, roles.field);
    ui.canvas.frame(area, roles.ink_dim);
    draw_text(ui.canvas, ui.font, left + 3, top + 3, text, roles.ink);
    ui.canvas.restore_clip(clip);
}
