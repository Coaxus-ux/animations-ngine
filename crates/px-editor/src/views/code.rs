use px_ui::{Area, Id, Key, Rect, Ui, draw_text, rect, text_width};

use super::Context;
use crate::app::{CodeBuffer, Intent};
use crate::domain::Document;

const GUTTER: i32 = 20;
/// Typing pauses this long before the text is loaded into the document.
const DEBOUNCE_MS: u64 = 300;
const INDENT: &str = "    ";

/// The scene's KDL, edited in place: after a pause the text loads into the document and the
/// canvas follows; while it doesn't load, the errors show below and the last good scene stays.
pub(super) fn code(ui: &mut Ui<'_>, context: &mut Context<'_>, area: Rect) {
    let Some(document) = context.document else {
        return;
    };
    let roles = ui.theme.roles;
    let errors = document.errors();
    let error_lines = i32::try_from(errors.len().min(3)).unwrap_or(0);
    let line_height = i32::from(ui.font.line_height());
    let (error_area, editor) =
        area.take_bottom(error_lines * line_height + i32::from(error_lines > 0) * 2);
    sync(context.code, document, context.editing.picked);
    let id = Id::new("code");
    let response = ui.interact(id, editor);
    if response.pressed {
        ui.state.focus = Some(id);
        if let Some(pointer) = ui.input.pointer {
            context.code.cursor = click(ui, context.code, editor, pointer);
        }
    } else if ui.input.pressed.left && !response.hovered && ui.state.focus == Some(id) {
        ui.state.focus = None;
    }
    let focused = ui.state.focus == Some(id);
    let visible = usize::try_from((editor.height / line_height.max(1)).max(1)).unwrap_or(1);
    if focused && type_keys(ui, context.code, visible) {
        context.code.edited_at = Some(ui.input.time_ms);
    }
    if response.hovered && ui.input.wheel.1 != 0.0 {
        let lines = context.code.text.lines().count();
        let down = ui.input.wheel.1 < 0.0;
        let scroll = &mut context.code.scroll;
        *scroll = if down {
            (*scroll + 3).min(lines.saturating_sub(1))
        } else {
            scroll.saturating_sub(3)
        };
    }
    if let Some(at) = context.code.edited_at
        && ui.input.time_ms.saturating_sub(at) >= DEBOUNCE_MS
    {
        context.code.edited_at = None;
        context.code.base.clone_from(&context.code.text);
        context
            .intents
            .push(Intent::SetText(context.code.text.clone()));
    }
    follow_cursor(ui, context.code, editor);
    if response.hovered && ui.input.wheel.0 != 0.0 {
        #[expect(clippy::cast_possible_truncation, reason = "a few wheel steps")]
        let steps = ui.input.wheel.0.round() as i32;
        context.code.scroll_x = (context.code.scroll_x - steps * 12).max(0);
    }
    draw(ui, context.code, editor, focused);
    ui.canvas.fill(error_area, roles.panel_dark);
    for (row, error) in (0i32..).zip(errors.iter().take(3)) {
        let y = error_area.y + 1 + row * line_height;
        draw_text(
            ui.canvas,
            ui.font,
            error_area.x + 2,
            y,
            error,
            roles.warning,
        );
    }
}

/// Takes the document's text when it changed elsewhere, and jumps to a newly picked object.
fn sync(code: &mut CodeBuffer, document: &Document, picked: Option<usize>) {
    if code.edited_at.is_none() && document.text() != code.base {
        document.text().clone_into(&mut code.text);
        code.base.clone_from(&code.text);
        code.cursor = code.cursor.min(code.text.chars().count());
    }
    if picked != code.jumped {
        code.jumped = picked;
        let offset = picked
            .and_then(|shape| document.loaded()?.origins.get(shape).copied())
            .filter(|origin| origin.file == 0)
            .map(|origin| origin.offset);
        if let Some(offset) = offset {
            code.cursor = code.text[..offset.min(code.text.len())].chars().count();
            let line = code.text[..offset.min(code.text.len())]
                .matches('\n')
                .count();
            code.scroll = line.saturating_sub(3);
        }
    }
}

/// Applies typed text and editing keys; returns whether the text changed.
fn type_keys(ui: &Ui<'_>, code: &mut CodeBuffer, visible: usize) -> bool {
    let mut changed = false;
    for character in ui.input.text.chars().filter(|c| !c.is_control()) {
        insert(code, &character.to_string());
        changed = true;
    }
    for key in &ui.input.keys {
        let (line, column) = position(&code.text, code.cursor);
        match key {
            Key::Enter => {
                let current = code.text.lines().nth(line).unwrap_or("");
                let mut indent: String = current.chars().take_while(|c| *c == ' ').collect();
                if current.trim_end().ends_with('{') {
                    indent.push_str(INDENT);
                }
                insert(code, &format!("\n{indent}"));
                changed = true;
            }
            Key::Tab => {
                insert(code, INDENT);
                changed = true;
            }
            Key::Backspace if code.cursor > 0 => {
                code.cursor -= 1;
                remove(code);
                changed = true;
            }
            Key::Delete if code.cursor < code.text.chars().count() => {
                remove(code);
                changed = true;
            }
            Key::Left => code.cursor = code.cursor.saturating_sub(1),
            Key::Right => code.cursor = (code.cursor + 1).min(code.text.chars().count()),
            Key::Up => code.cursor = index_of(&code.text, line.saturating_sub(1), column),
            Key::Down => code.cursor = index_of(&code.text, line + 1, column),
            Key::PageUp => code.cursor = index_of(&code.text, line.saturating_sub(visible), column),
            Key::PageDown => code.cursor = index_of(&code.text, line + visible, column),
            Key::Home => code.cursor = index_of(&code.text, line, 0),
            Key::End => code.cursor = index_of(&code.text, line, usize::MAX),
            _ => {}
        }
    }
    let (line, _) = position(&code.text, code.cursor);
    if line < code.scroll {
        code.scroll = line;
    } else if line >= code.scroll + visible {
        code.scroll = line + 1 - visible;
    }
    changed
}

fn insert(code: &mut CodeBuffer, text: &str) {
    let at = byte_index(&code.text, code.cursor);
    code.text.insert_str(at, text);
    code.cursor += text.chars().count();
}

fn remove(code: &mut CodeBuffer) {
    let at = byte_index(&code.text, code.cursor);
    if at < code.text.len() {
        code.text.remove(at);
    }
}

fn byte_index(text: &str, chars: usize) -> usize {
    text.char_indices()
        .nth(chars)
        .map_or(text.len(), |(index, _)| index)
}

/// (line, column) of character `cursor`.
fn position(text: &str, cursor: usize) -> (usize, usize) {
    let before: String = text.chars().take(cursor).collect();
    let line = before.matches('\n').count();
    let column = before
        .rsplit('\n')
        .next()
        .map_or(0, |last| last.chars().count());
    (line, column)
}

/// The character index of `column` on `line` (clamped to the line and the text).
fn index_of(text: &str, line: usize, column: usize) -> usize {
    let mut index = 0;
    for (number, content) in text.split('\n').enumerate() {
        let length = content.chars().count();
        if number == line {
            return index + column.min(length);
        }
        index += length + 1;
    }
    text.chars().count()
}

/// The character under a click.
fn click(ui: &Ui<'_>, code: &CodeBuffer, area: Rect, (x, y): (i32, i32)) -> usize {
    let line_height = i32::from(ui.font.line_height()).max(1);
    let line = code.scroll + usize::try_from((y - area.y) / line_height).unwrap_or(0);
    let content = code.text.split('\n').nth(line).unwrap_or("");
    let target = x - (area.x + GUTTER + 2) + code.scroll_x;
    let mut column = 0;
    let mut prefix = String::new();
    for character in content.chars() {
        prefix.push(character);
        if text_width(ui.font, &prefix) > target {
            break;
        }
        column += 1;
    }
    index_of(&code.text, line, column)
}

/// Scrolls sideways so the cursor stays in view.
fn follow_cursor(ui: &Ui<'_>, code: &mut CodeBuffer, area: Rect) {
    let (line, column) = position(&code.text, code.cursor);
    let content = code.text.split('\n').nth(line).unwrap_or("");
    let before: String = content.chars().take(column).collect();
    let x = text_width(ui.font, &before);
    let visible = area.width - GUTTER - 8;
    if x < code.scroll_x {
        code.scroll_x = (x - visible / 3).max(0);
    } else if x > code.scroll_x + visible {
        code.scroll_x = x - visible * 2 / 3;
    }
}

/// Line numbers, then each line colored: comments dim, strings in the accent.
fn draw(ui: &mut Ui<'_>, code: &CodeBuffer, area: Rect, focused: bool) {
    let roles = ui.theme.roles;
    ui.canvas.fill(area, roles.field);
    let (gutter, body) = area.take_left(GUTTER);
    ui.canvas.fill(gutter, roles.panel_dark);
    let line_height = i32::from(ui.font.line_height());
    let (cursor_line, cursor_column) = position(&code.text, code.cursor);
    let clip = ui.canvas.clip_to(area);
    for (row, (number, content)) in
        (0i32..).zip(code.text.split('\n').enumerate().skip(code.scroll))
    {
        let y = area.y + 1 + row * line_height;
        if y > area.bottom() {
            break;
        }
        draw_text(
            ui.canvas,
            ui.font,
            gutter.x + 1,
            y,
            &(number + 1).to_string(),
            roles.ink_dim,
        );
        let line_clip = ui.canvas.clip_to(body);
        let mut x = body.x + 2 - code.scroll_x;
        for (piece, kind) in tokens(content) {
            let paint = match kind {
                Token::Comment => roles.ink_dim,
                Token::Text => roles.accent_light,
                Token::Plain => roles.ink,
            };
            x = draw_text(ui.canvas, ui.font, x, y, piece, paint);
        }
        ui.canvas.restore_clip(line_clip);
        if focused && number == cursor_line && (ui.input.time_ms / 500).is_multiple_of(2) {
            let before: String = content.chars().take(cursor_column).collect();
            let cx = body.x + 2 + text_width(ui.font, &before) - code.scroll_x;
            ui.canvas
                .fill(rect(cx, y - 1, 1, line_height), roles.accent_light);
        }
    }
    ui.canvas.restore_clip(clip);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Token {
    Plain,
    Text,
    Comment,
}

/// Splits a line into plain text, quoted strings and a trailing `//` comment.
fn tokens(line: &str) -> Vec<(&str, Token)> {
    let mut pieces = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    for (index, character) in line.char_indices() {
        if !quoted && line[index..].starts_with("//") {
            pieces.push((&line[start..index], Token::Plain));
            pieces.push((&line[index..], Token::Comment));
            return pieces;
        }
        if character == '"' {
            let end = index + 1;
            let kind = if quoted { Token::Text } else { Token::Plain };
            let piece_end = if quoted { end } else { index };
            pieces.push((&line[start..piece_end], kind));
            start = piece_end;
            quoted = !quoted;
        }
    }
    pieces.push((
        &line[start..],
        if quoted { Token::Text } else { Token::Plain },
    ));
    pieces
}
