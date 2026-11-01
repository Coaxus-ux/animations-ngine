use px_ui::{Key, Ui};

use super::cursor::{byte_index, index_of, line_column};
use crate::app::CodeBuffer;

const INDENT: &str = "    ";

#[derive(Default)]
pub(super) struct Typed {
    pub changed: bool,
    pub copied: Option<String>,
}

pub(super) fn type_keys(ui: &Ui<'_>, code: &mut CodeBuffer, visible: usize) -> Typed {
    let mut typed = Typed::default();
    if ui.input.modifiers.command {
        clipboard_keys(ui, code, &mut typed);
        return typed;
    }
    for character in ui.input.text.chars().filter(|c| !c.is_control()) {
        replace_selection(code, &character.to_string());
        typed.changed = true;
    }
    for key in &ui.input.keys {
        if let Some(target) = travel(code, *key, visible) {
            code.anchor = if ui.input.modifiers.shift {
                code.anchor.or(Some(code.cursor))
            } else {
                None
            };
            code.cursor = target;
        } else {
            typed.changed |= edit(code, *key);
        }
    }
    typed
}

#[must_use]
pub(super) fn selection(code: &CodeBuffer) -> Option<(usize, usize)> {
    code.anchor
        .filter(|anchor| *anchor != code.cursor)
        .map(|anchor| (anchor.min(code.cursor), anchor.max(code.cursor)))
}

fn selected_text(code: &CodeBuffer) -> Option<String> {
    selection(code).map(|(start, end)| {
        code.text[byte_index(&code.text, start)..byte_index(&code.text, end)].to_owned()
    })
}

fn clipboard_keys(ui: &Ui<'_>, code: &mut CodeBuffer, typed: &mut Typed) {
    for key in &ui.input.keys {
        match key {
            Key::Letter('a') => {
                code.anchor = Some(0);
                code.cursor = code.text.chars().count();
            }
            Key::Letter('c') => typed.copied = selected_text(code).or(typed.copied.take()),
            Key::Letter('x') => {
                if let Some(text) = selected_text(code) {
                    typed.copied = Some(text);
                    typed.changed |= delete_selection(code);
                }
            }
            Key::Letter('v') => {
                if let Some(text) = &ui.input.clipboard {
                    replace_selection(code, &text.replace("\r\n", "\n"));
                    typed.changed = true;
                }
            }
            _ => {}
        }
    }
}

fn travel(code: &CodeBuffer, key: Key, visible: usize) -> Option<usize> {
    let (line, column) = line_column(&code.text, code.cursor);
    let text = &code.text;
    Some(match key {
        Key::Left => code.cursor.saturating_sub(1),
        Key::Right => (code.cursor + 1).min(text.chars().count()),
        Key::Up => index_of(text, line.saturating_sub(1), column),
        Key::Down => index_of(text, line + 1, column),
        Key::PageUp => index_of(text, line.saturating_sub(visible), column),
        Key::PageDown => index_of(text, line + visible, column),
        Key::Home => index_of(text, line, 0),
        Key::End => index_of(text, line, usize::MAX),
        _ => return None,
    })
}

fn edit(code: &mut CodeBuffer, key: Key) -> bool {
    match key {
        Key::Enter => {
            delete_selection(code);
            let (line, _) = line_column(&code.text, code.cursor);
            let current = code.text.split('\n').nth(line).unwrap_or("");
            let mut indent: String = current.chars().take_while(|c| *c == ' ').collect();
            if current.trim_end().ends_with('{') {
                indent.push_str(INDENT);
            }
            insert(code, &format!("\n{indent}"));
            true
        }
        Key::Tab => {
            replace_selection(code, INDENT);
            true
        }
        Key::Backspace => {
            delete_selection(code)
                || (code.cursor > 0 && {
                    code.cursor -= 1;
                    remove(code);
                    true
                })
        }
        Key::Delete => {
            delete_selection(code)
                || (code.cursor < code.text.chars().count() && {
                    remove(code);
                    true
                })
        }
        _ => false,
    }
}

fn replace_selection(code: &mut CodeBuffer, text: &str) {
    delete_selection(code);
    insert(code, text);
}

fn delete_selection(code: &mut CodeBuffer) -> bool {
    let Some((start, end)) = selection(code) else {
        code.anchor = None;
        return false;
    };
    let range = byte_index(&code.text, start)..byte_index(&code.text, end);
    code.text.replace_range(range, "");
    code.cursor = start;
    code.anchor = None;
    true
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
