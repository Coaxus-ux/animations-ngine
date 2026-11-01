use crate::area::{Area, Rect, rect};
use crate::input::Key;
use crate::text::text_width;
use crate::ui::{Id, Ui};
use crate::widgets::{button, label};

pub fn number_field(
    ui: &mut Ui<'_>,
    id: Id,
    area: Rect,
    value: &mut i64,
    range: (i64, i64),
) -> bool {
    let (min, max) = range;
    let before = *value;
    let (minus, rest) = area.take_left(area.height);
    let (plus, field) = rest.take_right(area.height);
    if button(ui, id.with_name("minus"), minus, "-") {
        *value -= 1;
    }
    if button(ui, id.with_name("plus"), plus, "+") {
        *value += 1;
    }
    if ui.hovering(field) {
        *value += wheel_step(ui.input.wheel.1);
    }
    let response = ui.interact(id, field);
    if response.pressed {
        ui.state.focus = Some(id);
        ui.state.drafts.insert(id, value.to_string());
    } else if ui.input.pressed.left && !response.hovered && ui.state.focus == Some(id) {
        ui.state.focus = None;
    }
    if ui.state.focus == Some(id) {
        edit_draft(ui, id);
    }
    let focused = ui.state.focus == Some(id);
    if !focused
        && let Some(draft) = ui.state.drafts.remove(&id)
        && let Ok(typed) = draft.parse::<i64>()
    {
        *value = typed;
    }
    *value = (*value).clamp(min, max);
    draw_field(ui, id, field, *value, focused);
    *value != before
}

fn edit_draft(ui: &mut Ui<'_>, id: Id) {
    let input = ui.input;
    let draft = ui.state.drafts.entry(id).or_default();
    for character in input.text.chars() {
        if character.is_ascii_digit() || (character == '-' && draft.is_empty()) {
            draft.push(character);
        }
    }
    for key in &input.keys {
        match key {
            Key::Backspace => {
                draft.pop();
            }
            Key::Enter => ui.state.focus = None,
            Key::Escape => {
                ui.state.drafts.remove(&id);
                ui.state.focus = None;
                return;
            }
            _ => {}
        }
    }
}

fn draw_field(ui: &mut Ui<'_>, id: Id, field: Rect, value: i64, focused: bool) {
    let roles = ui.theme.roles;
    ui.canvas.fill(field, roles.field);
    ui.canvas.frame(
        field,
        if focused {
            roles.accent
        } else {
            roles.panel_dark
        },
    );
    let text = ui
        .state
        .drafts
        .get(&id)
        .filter(|_| focused)
        .cloned()
        .unwrap_or_else(|| value.to_string());
    let inner = field.inset(2);
    let clip = ui.canvas.clip_to(inner);
    label(ui, inner, &text, roles.ink);
    if focused && (ui.input.time_ms / 500).is_multiple_of(2) {
        let x = inner.x + text_width(ui.font, &text) + 1;
        ui.canvas
            .fill(rect(x, inner.y, 1, inner.height), roles.accent_light);
    }
    ui.canvas.restore_clip(clip);
}

fn wheel_step(wheel: f32) -> i64 {
    if wheel > 0.0 {
        1
    } else if wheel < 0.0 {
        -1
    } else {
        0
    }
}
