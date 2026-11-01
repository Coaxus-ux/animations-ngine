use super::App;
use super::window::{Request, WindowKind};

impl App {
    pub(super) fn title_for(&self, kind: WindowKind) -> String {
        let name = self.document().map_or_else(
            || "px editor".to_owned(),
            |document| {
                let file = document
                    .path()
                    .file_name()
                    .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
                let dirty = if document.is_dirty() { " *" } else { "" };
                format!("{file}{dirty}")
            },
        );
        match kind {
            WindowKind::Main => format!("{name} - px editor"),
            WindowKind::Canvas => format!("{name} - view"),
            WindowKind::Panel(panel) => format!("{} - {name}", panel.title()),
        }
    }

    pub(super) fn retitle(&mut self) {
        let titles: Vec<_> = self
            .windows
            .iter()
            .map(|window| (window.id, self.title_for(window.kind)))
            .collect();
        for (id, title) in titles {
            self.requests.push(Request::Title { id, title });
        }
    }
}
