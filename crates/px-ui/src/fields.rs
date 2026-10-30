use crate::area::{Area, Rect, rect};
use crate::input::Key;
use crate::text::{draw_text, text_width};
use crate::ui::{Id, Ui};
use crate::widgets::label;

/// How a text field changed this frame.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FieldResponse {
    pub changed: bool,
    /// Enter was pressed (or focus left) after editing.
    pub submitted: bool,
    pub focused: bool,
}

/// A horizontal slider over `min..=max`, snapped to `step`; drag it or use the wheel.
/// Returns whether `value` changed.
pub fn slider(
    ui: &mut Ui<'_>,
    id: Id,
    area: Rect,
    value: &mut f64,
    range: (f64, f64),
    step: f64,
) -> bool {
    let (min, max) = range;
    let roles = ui.theme.roles;
    let response = ui.interact(id, area);
    let before = *value;
    if response.held
        && let Some((x, _)) = ui.input.pointer
    {
        let t = f64::from(x - area.x) / f64::from((area.width - 1).max(1));
        *value = min + t.clamp(0.0, 1.0) * (max - min);
    }
    if response.hovered {
        *value += f64::from(ui.input.wheel.1.signum()) * step;
    }
    if step > 0.0 {
        *value = (*value / step).round() * step;
    }
    *value = value.clamp(min, max);
    ui.canvas.fill(area, roles.field);
    ui.canvas.frame(area, roles.panel_dark);
    let t = if max > min {
        (*value - min) / (max - min)
    } else {
        0.0
    };
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a fraction of the slider's width"
    )]
    let filled = (t * f64::from(area.width - 2)).round() as i32;
    ui.canvas.fill(
        rect(area.x + 1, area.y + 1, filled, area.height - 2),
        roles.accent,
    );
    let text = if step >= 1.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    };
    let width = text_width(ui.font, &text);
    let y = area.y + (area.height - i32::from(ui.font.height())) / 2;
    draw_text(
        ui.canvas,
        ui.font,
        area.x + (area.width - width) / 2,
        y,
        &text,
        roles.ink,
    );
    (*value - before).abs() > f64::EPSILON
}

/// A one-line text field: click to focus, type, Enter to submit, Escape to leave.
pub fn text_field(ui: &mut Ui<'_>, id: Id, area: Rect, text: &mut String) -> FieldResponse {
    let roles = ui.theme.roles;
    let response = ui.interact(id, area);
    if response.pressed {
        ui.state.focus = Some(id);
        ui.state.cursor.insert(id, text.chars().count());
    } else if ui.input.pressed.left && !response.hovered && ui.state.focus == Some(id) {
        ui.state.focus = None;
        return FieldResponse {
            submitted: true,
            ..FieldResponse::default()
        };
    }
    let focused = ui.state.focus == Some(id);
    let mut result = FieldResponse {
        focused,
        ..FieldResponse::default()
    };
    if focused {
        let input = ui.input;
        let mut cursor = ui
            .state
            .cursor
            .get(&id)
            .copied()
            .unwrap_or(0)
            .min(text.chars().count());
        for character in input.text.chars().filter(|c| !c.is_control()) {
            let at = byte_index(text, cursor);
            text.insert(at, character);
            cursor += 1;
            result.changed = true;
        }
        for key in &input.keys {
            match key {
                Key::Backspace if cursor > 0 => {
                    cursor -= 1;
                    text.remove(byte_index(text, cursor));
                    result.changed = true;
                }
                Key::Delete if cursor < text.chars().count() => {
                    text.remove(byte_index(text, cursor));
                    result.changed = true;
                }
                Key::Left => cursor = cursor.saturating_sub(1),
                Key::Right => cursor = (cursor + 1).min(text.chars().count()),
                Key::Home => cursor = 0,
                Key::End => cursor = text.chars().count(),
                Key::Enter => {
                    result.submitted = true;
                    ui.state.focus = None;
                }
                Key::Escape => ui.state.focus = None,
                _ => {}
            }
        }
        ui.state.cursor.insert(id, cursor);
    }
    ui.canvas.fill(area, roles.field);
    ui.canvas.frame(
        area,
        if focused {
            roles.accent
        } else {
            roles.panel_dark
        },
    );
    let inner = area.inset(2);
    let clip = ui.canvas.clip_to(inner);
    label(ui, inner, text, roles.ink);
    if focused && (ui.input.time_ms / 500).is_multiple_of(2) {
        let cursor = ui.state.cursor.get(&id).copied().unwrap_or(0);
        let before: String = text.chars().take(cursor).collect();
        let x = inner.x + text_width(ui.font, &before) + 1;
        ui.canvas
            .fill(rect(x, inner.y, 1, inner.height), roles.accent_light);
    }
    ui.canvas.restore_clip(clip);
    result
}

fn byte_index(text: &str, chars: usize) -> usize {
    text.char_indices()
        .nth(chars)
        .map_or(text.len(), |(index, _)| index)
}
