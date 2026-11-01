mod cursor;
mod paint;
mod typing;

use std::borrow::Cow;
use std::collections::BTreeMap;

use px_format::edit::NodePath;
use px_ui::{Area, Id, Key, ROW, Rect, Ui, draw_text, header_button, label};

use super::Context;
use crate::app::{CodeBuffer, CodeMode, Intent};
use crate::domain::Document;
use crate::domain::tree::label_of;

const GUTTER: i32 = 20;
const DEBOUNCE_MS: u64 = 300;
const SCOPE_FIELD: &str = "code-node";

struct Scope<'a> {
    path: Option<&'a NodePath>,
    text: Cow<'a, str>,
    first_line: usize,
    header: Option<String>,
}

fn scope<'a>(document: &'a Document, selected: Option<&'a NodePath>) -> Scope<'a> {
    let source = document.source();
    if let Some(path) = selected
        && let Some(text) = source.node_text(path)
        && let Some((start, _)) = source.span(path)
        && let Some(node) = source.node(path)
    {
        return Scope {
            path: Some(path),
            text: Cow::Owned(text),
            first_line: source.text()[..start].matches('\n').count(),
            header: Some(format!("Editing {} (Esc: whole file)", label_of(node))),
        };
    }
    Scope {
        path: None,
        text: Cow::Borrowed(document.text()),
        first_line: 0,
        header: None,
    }
}

pub(super) fn pane_buttons(
    ui: &mut Ui<'_>,
    context: &mut Context<'_>,
    id: Id,
    header: Rect,
) -> i32 {
    let tip = "Close the code (F2 brings it back)";
    if header_button(ui, id.with_name("close"), header, 0, "close", tip) {
        context.intents.push(Intent::SetCode(CodeMode::Hidden));
    }
    let (icon, tip, mode) = if context.view.code == CodeMode::Only {
        (
            "restore",
            "Show the canvas next to the code",
            CodeMode::Split,
        )
    } else {
        ("maximize", "Code only", CodeMode::Only)
    };
    if header_button(ui, id.with_name("size"), header, 1, icon, tip) {
        context.intents.push(Intent::SetCode(mode));
    }
    2
}

pub(super) fn code(ui: &mut Ui<'_>, context: &mut Context<'_>, area: Rect) {
    let Some(document) = context.document else {
        return;
    };
    let roles = ui.theme.roles;
    let scope = scope(document, context.selected);
    let area = header(ui, context, &scope, area);
    let area = super::preview::strip(ui, context, area);
    let errors = document.errors();
    let error_lines = i32::try_from(errors.len().min(3)).unwrap_or(0);
    let line_height = i32::from(ui.font.line_height());
    let (error_area, editor) =
        area.take_bottom(error_lines * line_height + i32::from(error_lines > 0) * 2);
    sync(
        context.code,
        context.fields,
        &scope,
        document,
        context.editing.picked,
    );
    let id = Id::new("code");
    let response = ui.interact(id, editor);
    let before = context.code.cursor;
    if response.pressed {
        ui.state.focus = Some(id);
        if let Some(pointer) = ui.input.pointer {
            let at = cursor::click(ui, context.code, editor, pointer);
            let code = &mut *context.code;
            code.anchor = if ui.input.modifiers.shift {
                code.anchor.or(Some(code.cursor))
            } else {
                Some(at)
            };
            code.cursor = at;
        }
    } else if response.held
        && let Some(pointer) = ui.input.pointer
    {
        context.code.cursor = cursor::click(ui, context.code, editor, pointer);
    } else if ui.input.pressed.left && !response.hovered && ui.state.focus == Some(id) {
        ui.state.focus = None;
    }
    let focused = ui.state.focus == Some(id);
    let visible = usize::try_from((editor.height / line_height.max(1)).max(1)).unwrap_or(1);
    let typed = if focused {
        typing::type_keys(ui, context.code, visible)
    } else {
        typing::Typed::default()
    };
    if typed.changed {
        context.code.edited_at = Some(ui.input.time_ms);
    }
    if let Some(text) = typed.copied {
        context.intents.push(Intent::CopyText(text));
    }
    if response.hovered {
        wheel(ui, context.code, id, visible);
    }
    if let Some(at) = context.code.edited_at
        && ui.input.time_ms.saturating_sub(at) >= DEBOUNCE_MS
    {
        context.code.edited_at = None;
        context.code.base.clone_from(&context.code.text);
        let text = context.code.text.clone();
        context.intents.push(match scope.path {
            Some(path) => Intent::ReplaceNode {
                path: path.clone(),
                text,
            },
            None => Intent::SetText(text),
        });
    }
    if typed.changed || context.code.cursor != before {
        cursor::reveal(ui, context.code, editor, visible);
    }
    paint::draw(ui, context.code, editor, focused, scope.first_line);
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

fn wheel(ui: &mut Ui<'_>, code: &mut CodeBuffer, id: Id, visible: usize) {
    let line_height = i32::from(ui.font.line_height());
    let lines = ui.wheel_steps(id.with_name("lines"), line_height).1;
    let pixels = ui.wheel_steps(id.with_name("pixels"), 1).0;
    let last = code.text.split('\n').count().saturating_sub(visible);
    let moved = i64::try_from(code.scroll).unwrap_or(0) + i64::from(lines);
    code.scroll = usize::try_from(moved.max(0)).unwrap_or(0).min(last);
    code.scroll_x = (code.scroll_x + pixels).max(0);
}

fn header(ui: &mut Ui<'_>, context: &mut Context<'_>, scope: &Scope<'_>, area: Rect) -> Rect {
    let Some(title) = &scope.header else {
        return area;
    };
    let roles = ui.theme.roles;
    let (header, rest) = area.take_top(ROW);
    ui.canvas.fill(header, roles.panel_dark);
    label(ui, header, &format!(" {title}"), roles.ink_dim);
    if ui.input.key(Key::Escape) && !ui.state.modal {
        context.intents.push(Intent::SelectElement(None));
    }
    rest
}

fn sync(
    code: &mut CodeBuffer,
    fields: &mut BTreeMap<Id, String>,
    scope: &Scope<'_>,
    document: &Document,
    picked: Option<usize>,
) {
    let field = Id::new(SCOPE_FIELD);
    let key = scope.path.map(ToString::to_string);
    if fields.get(&field) != key.as_ref() {
        match key {
            Some(key) => fields.insert(field, key),
            None => fields.remove(&field),
        };
        if *scope.text != code.text {
            code.text = scope.text.to_string();
            code.cursor = 0;
            code.anchor = None;
            code.scroll = 0;
            code.scroll_x = 0;
        }
        code.base.clone_from(&code.text);
        code.edited_at = None;
    } else if code.edited_at.is_none() && *scope.text != code.base {
        code.text = scope.text.to_string();
        code.base.clone_from(&code.text);
        code.cursor = code.cursor.min(code.text.chars().count());
        code.anchor = None;
    }
    if picked != code.jumped {
        code.jumped = picked;
        if scope.path.is_none() {
            jump(code, document, picked);
        }
    }
}

fn jump(code: &mut CodeBuffer, document: &Document, picked: Option<usize>) {
    let offset = picked
        .and_then(|shape| document.loaded()?.origins.get(shape).copied())
        .filter(|origin| origin.file == 0)
        .map(|origin| origin.offset.min(code.text.len()));
    if let Some(offset) = offset {
        code.cursor = code.text[..offset].chars().count();
        code.anchor = None;
        let line = code.text[..offset].matches('\n').count();
        code.scroll = line.saturating_sub(3);
    }
}
