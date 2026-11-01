use crate::area::{Area, Rect, rect};
use crate::input::Key;
use crate::ui::{Id, Response, Ui};
use crate::widgets::{ROW, SMALL_ICON, label};

pub(crate) const SCROLLBAR: i32 = 5;
const WHEEL_STEP: i32 = ROW;

pub fn scroll_area(ui: &mut Ui<'_>, id: Id, area: Rect, content_height: i32) -> (Rect, i32) {
    let roles = ui.theme.roles;
    let overflow = (content_height - area.height).max(0);
    let mut offset = ui.state.scroll.get(&id).copied().unwrap_or(0);
    if ui.hovering(area) {
        #[expect(clippy::cast_possible_truncation, reason = "a few wheel steps")]
        let steps = ui.input.wheel.1.round() as i32;
        offset -= steps * WHEEL_STEP;
    }
    if overflow == 0 {
        ui.state.scroll.insert(id, 0);
        return (area, 0);
    }
    let (bar, view) = area.take_right(SCROLLBAR);
    let thumb_height = (bar.height * area.height / content_height.max(1)).max(4);
    let response = ui.interact(id.with_name("thumb"), bar);
    if response.held
        && let Some((_, y)) = ui.input.pointer
    {
        let t =
            f64::from(y - bar.y - thumb_height / 2) / f64::from((bar.height - thumb_height).max(1));
        #[expect(clippy::cast_possible_truncation, reason = "within the scroll range")]
        let target = (t.clamp(0.0, 1.0) * f64::from(overflow)).round() as i32;
        offset = target;
    }
    offset = offset.clamp(0, overflow);
    ui.state.scroll.insert(id, offset);
    ui.canvas.fill(bar, roles.panel_dark);
    let thumb_y = bar.y + (bar.height - thumb_height) * offset / overflow.max(1);
    let thumb = rect(bar.x + 1, thumb_y, bar.width - 2, thumb_height);
    ui.canvas.fill(
        thumb,
        if response.held {
            roles.accent
        } else {
            roles.panel_light
        },
    );
    (view, offset)
}

pub fn list_row(ui: &mut Ui<'_>, id: Id, area: Rect, text: &str, selected: bool) -> Response {
    let response = row_face(ui, id, area, selected);
    row_text(ui, area, text, selected);
    response
}

pub fn list_row_icon(
    ui: &mut Ui<'_>,
    id: Id,
    area: Rect,
    icon: &str,
    text: &str,
    selected: bool,
) -> Response {
    let response = row_face(ui, id, area, selected);
    let (slot, rest) = area.take_left(SMALL_ICON + 2);
    if let Some(bitmap) = ui.icons.get(icon) {
        let y = slot.y + (slot.height - bitmap.height()) / 2;
        let tint = (!selected).then_some(ui.theme.roles.ink_dim);
        ui.canvas.blit(bitmap, slot.x + 1, y, tint);
    }
    row_text(ui, rest, text, selected);
    response
}

fn row_face(ui: &mut Ui<'_>, id: Id, area: Rect, selected: bool) -> Response {
    let roles = ui.theme.roles;
    let focused = ui.focusable(id);
    let mut response = ui.interact(id, area);
    if selected {
        ui.canvas.fill(area, roles.accent);
    } else if response.hovered {
        ui.canvas.dither(area, roles.panel_light);
    }
    if focused {
        ui.focus_ring(area);
        response.clicked |= ui.activated(true);
        if ui.take_key(Key::Down) {
            ui.step_focus(false);
        } else if ui.take_key(Key::Up) {
            ui.step_focus(true);
        }
    }
    response
}

fn row_text(ui: &mut Ui<'_>, area: Rect, text: &str, selected: bool) {
    let roles = ui.theme.roles;
    let ink = if selected { roles.ink } else { roles.ink_dim };
    label(
        ui,
        area.take_left(area.width - 2).0,
        &format!(" {text}"),
        ink,
    );
}

#[must_use]
pub fn drop_index(top: i32, row: i32, count: usize, y: i32) -> usize {
    let slot = ((y - top + row / 2) / row.max(1)).max(0);
    usize::try_from(slot).unwrap_or(0).min(count)
}
