use kdl::{KdlEntry, KdlValue};
use px_format::edit::NodePath;
use px_ui::{Area, Id, ROW, Rect, Ui, label, rect, scroll_area, text_field, toggle_icon};

use super::{Context, unavailable};
use crate::app::Intent;
use crate::domain::{Document, scene_path};

const KEY: i32 = 58;

pub(super) fn inspector(ui: &mut Ui<'_>, context: &mut Context<'_>, area: Rect) {
    let Some(document) = context.document else {
        unavailable(ui, context, area);
        return;
    };
    let roles = ui.theme.roles;
    let Some(path) = target(document, context.selected, context.editing.picked) else {
        label(
            ui,
            area.take_top(ROW).0,
            " Pick an object (O)",
            roles.ink_dim,
        );
        return;
    };
    let Some(node) = document.source().node(&path) else {
        return;
    };
    let (title, rest) = area.take_top(ROW + 2);
    let id_text = node
        .entries()
        .first()
        .filter(|entry| entry.name().is_none())
        .and_then(|entry| entry.value().as_string())
        .map_or_else(String::new, |id| format!(" \"{id}\""));
    label(
        ui,
        title,
        &format!(" {}{id_text}", node.name().value()),
        roles.ink,
    );
    let entries: Vec<&KdlEntry> = node
        .entries()
        .iter()
        .filter(|entry| entry.name().is_some())
        .collect();
    let id = Id::new("inspector").with_name(&path.to_string());
    let total_height = i32::try_from(entries.len()).unwrap_or(0) * (ROW + 1);
    let (view, offset) = scroll_area(ui, id, rest, total_height);
    let clip = ui.canvas.clip_to(view);
    for (slot, entry) in (0i32..).zip(entries) {
        let Some(key) = entry.name().map(|name| name.value().to_owned()) else {
            continue;
        };
        let row = rect(view.x, view.y + slot * (ROW + 1) - offset, view.width, ROW);
        let (caption, field) = row.take_left(KEY);
        label(ui, caption, &format!(" {key}"), roles.ink_dim);
        if ui.hovering(caption) {
            ui.tooltip(&key);
        }
        property(
            ui,
            context,
            id.with_name(&key),
            field,
            (&path, &key),
            entry.value(),
        );
    }
    ui.canvas.restore_clip(clip);
}

fn target(
    document: &Document,
    selected: Option<&NodePath>,
    picked: Option<usize>,
) -> Option<NodePath> {
    if let Some(path) = selected {
        return Some(path.clone());
    }
    let loaded = document.loaded()?;
    let picked = picked
        .and_then(|shape| loaded.origins.get(shape))
        .filter(|origin| origin.file == 0)
        .and_then(|origin| document.source().path_at(origin.offset));
    picked.or_else(|| Some(scene_path(&loaded.scene)))
}

fn property(
    ui: &mut Ui<'_>,
    context: &mut Context<'_>,
    id: Id,
    area: Rect,
    (path, key): (&NodePath, &str),
    value: &KdlValue,
) {
    if let Some(on) = value.as_bool() {
        if toggle_icon(ui, id, area.take_left(ROW).0, on, ("eye", "eye-off"), "") {
            context.intents.push(Intent::SetProperty {
                path: path.clone(),
                key: key.to_owned(),
                value: KdlValue::Bool(!on),
            });
        }
        return;
    }
    let shown = match value {
        KdlValue::String(text) => text.clone(),
        other => other.to_string(),
    };
    let mut text = context
        .fields
        .get(&id)
        .cloned()
        .unwrap_or_else(|| shown.clone());
    let response = text_field(ui, id, area.inset(0), &mut text);
    if response.focused || response.changed {
        context.fields.insert(id, text.clone());
    }
    if response.submitted {
        context.fields.remove(&id);
        if text != shown {
            context.intents.push(Intent::SetProperty {
                path: path.clone(),
                key: key.to_owned(),
                value: parse(&text, value),
            });
        }
    }
}

fn parse(text: &str, before: &KdlValue) -> KdlValue {
    match before {
        KdlValue::Integer(_) | KdlValue::Float(_) => text
            .parse::<i128>()
            .map(KdlValue::Integer)
            .or_else(|_| text.parse::<f64>().map(KdlValue::Float))
            .unwrap_or_else(|_| KdlValue::String(text.to_owned())),
        _ => KdlValue::String(text.to_owned()),
    }
}
