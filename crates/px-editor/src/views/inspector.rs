use kdl::{KdlNode, KdlValue};
use px_format::edit::{NodePath, coordinates_of, id_of};
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
    let id_text = id_of(node).map_or_else(String::new, |id| format!(" \"{id}\""));
    label(
        ui,
        title,
        &format!(" {}{id_text}", node.name().value()),
        roles.ink,
    );
    let rows = rows(document, node);
    let id = Id::new("inspector").with_name(&path.to_string());
    let total_height = i32::try_from(rows.len()).unwrap_or(0) * (ROW + 1);
    let (view, offset) = scroll_area(ui, id, rest, total_height);
    let clip = ui.canvas.clip_to(view);
    for (slot, item) in (0i32..).zip(rows) {
        let row = rect(view.x, view.y + slot * (ROW + 1) - offset, view.width, ROW);
        let (caption, field) = row.take_left(KEY);
        label(ui, caption, &format!(" {}", item.label), roles.ink_dim);
        if ui.hovering(caption) {
            ui.tooltip(&item.label);
        }
        property(ui, context, id.with_name(&item.label), field, &path, &item);
    }
    ui.canvas.restore_clip(clip);
}

#[derive(Clone, Copy)]
enum Write {
    Property,
    Argument(usize),
}

struct Row<'n> {
    label: String,
    write: Write,
    value: &'n KdlValue,
}

fn rows<'n>(document: &Document, node: &'n KdlNode) -> Vec<Row<'n>> {
    let name = node.name().value();
    let coordinates = coordinates_of(name);
    let (skipped, labels): (usize, Vec<String>) = if coordinates.is_empty() {
        let motif = document
            .loaded()
            .and_then(|loaded| loaded.motifs.iter().find(|motif| motif.name == name));
        let labels = motif.map_or_else(Vec::new, |motif| {
            motif.positional().map(str::to_owned).collect()
        });
        (0, labels)
    } else {
        let labels = coordinates.iter().map(|slot| (*slot).to_owned()).collect();
        (usize::from(id_of(node).is_some()), labels)
    };
    let arguments = node
        .entries()
        .iter()
        .filter(|entry| entry.name().is_none())
        .enumerate()
        .skip(skipped)
        .zip(labels)
        .map(|((index, entry), label)| Row {
            label,
            write: Write::Argument(index),
            value: entry.value(),
        });
    let properties = node.entries().iter().filter_map(|entry| {
        Some(Row {
            label: entry.name()?.value().to_owned(),
            write: Write::Property,
            value: entry.value(),
        })
    });
    arguments.chain(properties).collect()
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
    path: &NodePath,
    row: &Row<'_>,
) {
    if let Some(on) = row.value.as_bool() {
        if toggle_icon(ui, id, area.take_left(ROW).0, on, ("eye", "eye-off"), "") {
            context.intents.push(submit(path, row, !on));
        }
        return;
    }
    let shown = match row.value {
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
            context.intents.push(typed(path, row, text));
        }
    }
}

fn submit(path: &NodePath, row: &Row<'_>, on: bool) -> Intent {
    match row.write {
        Write::Argument(index) => Intent::SetArgument {
            path: path.clone(),
            index,
            text: if on { "#true" } else { "#false" }.to_owned(),
        },
        Write::Property => Intent::SetProperty {
            path: path.clone(),
            key: row.label.clone(),
            value: KdlValue::Bool(on),
        },
    }
}

fn typed(path: &NodePath, row: &Row<'_>, text: String) -> Intent {
    match row.write {
        Write::Argument(index) => Intent::SetArgument {
            path: path.clone(),
            index,
            text,
        },
        Write::Property => Intent::SetProperty {
            path: path.clone(),
            key: row.label.clone(),
            value: parse(&text, row.value),
        },
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
