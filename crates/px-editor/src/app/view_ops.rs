use super::window::WindowKind;
use super::{App, WindowId};
use crate::views::Placement;

const MAX_ZOOM: i32 = 64;

impl App {
    pub(super) fn zoom(&mut self, window: WindowId, steps: i32, at: (i32, i32)) {
        let canvas = self.canvas_size();
        let Some(window) = self.windows.iter_mut().find(|other| other.id == window) else {
            return;
        };
        let Some(area) = window.area else { return };
        let placement = Placement::of(&window.view, area, canvas);
        let zoom = (placement.zoom + steps).clamp(1, MAX_ZOOM);
        let point = placement.to_canvas(at.0, at.1);
        let origin = (at.0 - point.0 * zoom, at.1 - point.1 * zoom);
        window.view.zoom = zoom;
        window.view.pan = Some((origin.0 - area.x, origin.1 - area.y));
    }

    pub(super) fn pan(&mut self, window: WindowId, dx: i32, dy: i32) {
        let canvas = self.canvas_size();
        let Some(window) = self.windows.iter_mut().find(|other| other.id == window) else {
            return;
        };
        let Some(area) = window.area else { return };
        let placement = Placement::of(&window.view, area, canvas);
        window.view.zoom = placement.zoom;
        window.view.pan = Some((
            placement.origin.0 - area.x + dx,
            placement.origin.1 - area.y + dy,
        ));
    }

    pub(super) fn pop_out(&mut self, panel: super::PanelKind) {
        let open = self
            .windows
            .iter()
            .any(|window| window.kind == WindowKind::Panel(panel));
        if let Some(main) = self.main_mut() {
            main.popped.insert(panel);
        }
        if !open {
            self.open_window(WindowKind::Panel(panel));
        }
    }

    pub(super) fn dock(&mut self, panel: super::PanelKind) {
        let id = self
            .windows
            .iter()
            .find(|window| window.kind == WindowKind::Panel(panel))
            .map(|window| window.id);
        if let Some(id) = id {
            self.requests.push(super::Request::Close { id });
            self.window_closed(id);
        }
    }

    pub(super) fn layer(&self, index: usize) -> Option<&crate::domain::Layer> {
        self.document.as_ref()?.layers().get(index)
    }

    pub(super) fn duration(&self) -> u32 {
        self.document
            .as_ref()
            .and_then(crate::domain::Document::scene)
            .map_or(1, |scene| scene.timing().duration.get().max(1))
    }

    pub(super) fn canvas_size(&self) -> (i32, i32) {
        self.document
            .as_ref()
            .and_then(crate::domain::Document::scene)
            .map_or((1, 1), |scene| {
                (
                    i32::from(scene.size().width()),
                    i32::from(scene.size().height()),
                )
            })
    }
}
