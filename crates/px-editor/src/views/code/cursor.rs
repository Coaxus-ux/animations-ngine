use px_ui::{Rect, Ui, text_width};

use super::GUTTER;
use crate::app::CodeBuffer;

pub(super) fn byte_index(text: &str, chars: usize) -> usize {
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

pub(super) fn index_of(text: &str, line: usize, column: usize) -> usize {
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

pub(super) fn reveal(ui: &Ui<'_>, code: &mut CodeBuffer, area: Rect, visible: usize) {
    let (line, column) = line_column(&code.text, code.cursor);
    if line < code.scroll {
        code.scroll = line;
    } else if line >= code.scroll + visible {
        code.scroll = line + 1 - visible;
    }
    let content = code.text.split('\n').nth(line).unwrap_or("");
    let before: String = content.chars().take(column).collect();
    let x = text_width(ui.font, &before);
    let width = area.width - GUTTER - 8;
    if x < code.scroll_x {
        code.scroll_x = (x - width / 3).max(0);
    } else if x > code.scroll_x + width {
        code.scroll_x = x - width * 2 / 3;
    }
}
