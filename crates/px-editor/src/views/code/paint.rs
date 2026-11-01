use px_ui::{Area, Rect, Ui, draw_text, rect, text_width};

use super::GUTTER;
use super::cursor::line_column;
use crate::app::CodeBuffer;

/// Line numbers from `first_line + 1`, then each line colored: comments dim, strings in the
/// accent.
pub(super) fn draw(
    ui: &mut Ui<'_>,
    code: &CodeBuffer,
    area: Rect,
    focused: bool,
    first_line: usize,
) {
    let roles = ui.theme.roles;
    ui.canvas.fill(area, roles.field);
    let (gutter, body) = area.take_left(GUTTER);
    ui.canvas.fill(gutter, roles.panel_dark);
    let line_height = i32::from(ui.font.line_height());
    let (cursor_line, cursor_column) = line_column(&code.text, code.cursor);
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
            &(first_line + number + 1).to_string(),
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
