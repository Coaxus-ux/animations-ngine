use std::path::PathBuf;

use super::Context;
use super::menus::{Entry, entry, separator};
use crate::app::{DialogKind, Intent};

pub(super) fn file<'a>(context: &Context<'_>, recent: &'a [String]) -> Vec<Entry<'a>> {
    let open = |intent: Intent| context.document.map(|_| intent);
    let mut entries = vec![
        entry("New", "Ctrl+N", Some(Intent::ShowDialog(DialogKind::New))),
        entry("Open", "Ctrl+O", Some(Intent::OpenFile(PathBuf::new()))),
        separator(),
    ];
    if recent.is_empty() {
        entries.push(entry("No recent files", "", None));
    }
    for (label, path) in recent.iter().zip(context.recent) {
        entries.push(entry(label, "", Some(Intent::OpenFile(path.clone()))));
    }
    entries.push(entry(
        "Clear recent",
        "",
        (!recent.is_empty()).then_some(Intent::ClearRecent),
    ));
    entries.extend([
        separator(),
        entry("Save", "Ctrl+S", open(Intent::Save)),
        entry(
            "Save As",
            "Ctrl+Shift+S",
            open(Intent::SaveAs(PathBuf::new())),
        ),
        entry(
            "Close",
            "Ctrl+W",
            context.active_tab.map(|index| Intent::CloseTab {
                index,
                discard: false,
            }),
        ),
        entry("Close All", "Ctrl+Shift+W", open(Intent::CloseAll)),
        separator(),
        entry(
            "Export As",
            "Ctrl+Alt+Shift+S",
            open(Intent::ShowDialog(DialogKind::Export)),
        ),
        entry(
            "Export Sprite Sheet",
            "Ctrl+E",
            open(Intent::ShowDialog(DialogKind::ExportSheet)),
        ),
        entry(
            "Repeat Last Export",
            "Ctrl+Shift+X",
            open(Intent::RepeatExport),
        ),
        separator(),
        entry("New canvas window", "Ctrl+Shift+N", open(Intent::NewWindow)),
        entry("Quit", "Ctrl+Q", Some(Intent::Quit)),
    ]);
    entries
}

pub(super) fn sprite<'a>(context: &Context<'_>) -> Vec<Entry<'a>> {
    let open = |intent: Intent| context.document.map(|_| intent);
    vec![
        entry(
            "Canvas Size",
            "C",
            open(Intent::ShowDialog(DialogKind::CanvasSize)),
        ),
        entry(
            "Properties",
            "Ctrl+P",
            open(Intent::ShowDialog(DialogKind::Properties)),
        ),
    ]
}
