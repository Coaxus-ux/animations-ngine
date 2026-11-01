use std::fmt::Write as _;
use std::path::PathBuf;

use kdl::{KdlDocument, KdlNode};

use super::App;
use super::window::{PanelKind, WindowKind};
use crate::assets::ThemeChoice;
use crate::domain::{ExportJob, Recent};

const MIN_SCALE: i32 = 1;
const MAX_SCALE: i32 = 4;

impl App {
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
        text.push_str(&self.recent.to_kdl());
        if let Some(folder) = &self.last_folder {
            let _ = writeln!(text, "    folder {:?}", folder.to_string_lossy());
        }
        if let Some(export) = &self.last_export {
            text.push_str(&export.to_kdl());
        }
        text.push_str("}\n");
        text
    }

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
        self.recent = Recent::from_node(editor);
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
                            &mut crate::ports::NoFiles,
                        );
                    }
                }
                "folder" => {
                    self.last_folder = node
                        .entries()
                        .first()
                        .and_then(|entry| entry.value().as_string())
                        .map(PathBuf::from);
                }
                "export" => self.last_export = ExportJob::from_node(node),
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

    pub fn set_scale(&mut self, scale: Option<i32>) {
        self.scale = scale.map(|scale| scale.clamp(MIN_SCALE, MAX_SCALE));
    }
}

fn integer(node: &KdlNode, key: &str) -> Option<i128> {
    node.entry(key).and_then(|entry| entry.value().as_integer())
}
