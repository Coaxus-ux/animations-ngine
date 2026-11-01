use px_ui::{Key, Rect, Ui, text_width};

use super::GUTTER;
use crate::app::CodeBuffer;

const INDENT: &str = "    ";

/// Applies typed text and editing keys; returns whether the text changed.
pub(super) fn type_keys(ui: &Ui<'_>, code: &mut CodeBuffer, visible: usize) -> bool {
    let mut changed = false;
    for character in ui.input.text.chars().filter(|c| !c.is_control()) {
        insert(code, &character.to_string());
        changed = true;
    }
    for key in &ui.input.keys {
        let (line, column) = line_column(&code.text, code.cursor);
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
    let (line, _) = line_column(&code.text, code.cursor);
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

pub(super) fn line_column(text: &str, cursor: usize) -> (usize, usize) {
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

pub(super) fn click(ui: &Ui<'_>, code: &CodeBuffer, area: Rect, (x, y): (i32, i32)) -> usize {
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
pub(super) fn follow(ui: &Ui<'_>, code: &mut CodeBuffer, area: Rect) {
    let (line, column) = line_column(&code.text, code.cursor);
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
