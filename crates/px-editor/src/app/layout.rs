use std::fmt::Write as _;

use kdl::{KdlDocument, KdlNode};

use super::App;
use super::window::{PanelKind, WindowKind};
use crate::assets::ThemeChoice;

const MIN_SCALE: i32 = 1;
const MAX_SCALE: i32 = 4;

impl App {
    /// The workspace layout, as KDL, for the adapter to keep per user: theme, UI scale,
    /// torn-off panels and extra canvas windows with their zoom.
    #[must_use]
    pub fn layout_text(&self) -> String {
        let theme = match self.theme {
            ThemeChoice::Dark => "dark",
            ThemeChoice::Light => "light",
        };
        let scale = self
            .scale
            .map_or_else(|| "\"auto\"".to_owned(), |scale| scale.to_string());
        let mut text = format!("editor theme=\"{theme}\" scale={scale} {{\n");
        for window in &self.windows {
            match window.kind {
                WindowKind::Panel(panel) => {
                    let _ = writeln!(text, "    panel \"{}\"", panel.title());
                }
                WindowKind::Canvas => {
                    let _ = writeln!(text, "    canvas zoom={}", window.view.zoom);
                }
                WindowKind::Main => {}
            }
        }
        text.push_str("}\n");
        text
    }

    /// Restores a layout written by `layout_text`; anything unreadable is skipped.
    pub fn apply_layout(&mut self, text: &str) {
        let Ok(document) = KdlDocument::parse_v2(text) else {
            return;
        };
        let Some(editor) = document
            .nodes()
            .iter()
            .find(|node| node.name().value() == "editor")
        else {
            return;
        };
        self.scale = integer(editor, "scale")
            .and_then(|scale| i32::try_from(scale).ok())
            .map(|scale| scale.clamp(MIN_SCALE, MAX_SCALE));
        let theme = match editor
            .entry("theme")
            .and_then(|entry| entry.value().as_string())
        {
            Some("light") => ThemeChoice::Light,
            _ => ThemeChoice::Dark,
        };
        if theme != self.theme {
            self.theme = theme;
            self.reload_assets();
        }
        for node in editor
            .children()
            .map(KdlDocument::nodes)
            .unwrap_or_default()
        {
            match node.name().value() {
                "panel" => {
                    let panel = node
                        .entries()
                        .first()
                        .and_then(|entry| entry.value().as_string())
                        .and_then(PanelKind::from_title);
                    if let Some(panel) = panel {
                        self.apply(
                            super::Intent::PopOut(panel),
                            self.focused,
                            &px_ui::Input::default(),
                            &mut NoFiles,
                        );
                    }
                }
                "canvas" => {
                    let id = self.open_window(WindowKind::Canvas);
                    let zoom = integer(node, "zoom").and_then(|zoom| i32::try_from(zoom).ok());
                    if let Some(window) = self.windows.iter_mut().find(|window| window.id == id) {
                        window.view.zoom = zoom.unwrap_or(0).max(0);
                    }
                }
                _ => {}
            }
        }
    }

    /// Sets window pixels per UI pixel (1..=4), or `None` to choose by window size.
    pub fn set_scale(&mut self, scale: Option<i32>) {
        self.scale = scale.map(|scale| scale.clamp(MIN_SCALE, MAX_SCALE));
    }
}

fn integer(node: &KdlNode, key: &str) -> Option<i128> {
    node.entry(key).and_then(|entry| entry.value().as_integer())
}

/// Restoring a layout never touches files.
struct NoFiles;

impl crate::ports::FileStore for NoFiles {
    fn read(&mut self, _path: &std::path::Path) -> Result<String, String> {
        Err("no files while restoring a layout".to_owned())
    }

    fn write(&mut self, _path: &std::path::Path, _text: &str) -> Result<(), String> {
        Err("no files while restoring a layout".to_owned())
    }

    fn stamp(&mut self, _path: &std::path::Path) -> Option<u64> {
        None
    }
}
