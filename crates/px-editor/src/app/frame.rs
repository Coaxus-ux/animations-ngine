use px_ui::{Input, Ui, present, present_top};

use super::window::WindowKind;
use super::{App, WindowId, shortcuts};
use crate::ports::FileStore;
use crate::views::{self, Context};

const DISK_CHECK_MS: u64 = 500;

impl App {
    pub fn frame(
        &mut self,
        id: WindowId,
        input: &Input,
        size: (usize, usize),
        store: &mut dyn FileStore,
    ) -> Option<&[u32]> {
        let index = self.windows.iter().position(|window| window.id == id)?;
        let mut intents = Vec::new();
        let kind = self.windows[index].kind;
        let ui_state = &self.windows[index].ui;
        let typing = ui_state.focus.is_some();
        let navigating = ui_state.key_focus.is_some() || ui_state.open_menu.is_some();
        let modal = self.windows[index].dialog.is_some();
        let tabs = (self.documents.active_index(), self.documents.len());
        shortcuts::collect(input, (typing, navigating), modal, tabs, &mut intents);
        self.tick(input.time_ms);
        if kind == WindowKind::Main
            && input.time_ms % DISK_CHECK_MS < 20
            && let Some(document) = self.document_mut()
        {
            document.check_disk(store);
        }
        if input.pointer.is_some()
            && kind.has_canvas()
            && (input.pressed.left || input.pressed.right)
        {
            self.focused = id;
        }

        let mut window = self.windows.remove(index);
        let view = if kind.has_canvas() {
            window.view.clone()
        } else {
            self.windows
                .iter()
                .find(|other| other.id == self.focused)
                .map_or_else(|| window.view.clone(), |focused| focused.view.clone())
        };
        let mut cache = self.caches.remove(&id).unwrap_or_default();
        if let Some(slot) = self.documents.active() {
            cache.update(slot.id, &slot.document, &view, self.editing.playing);
        }
        self.draw_window(&mut window, &view, &cache, input, size, &mut intents);
        self.caches.insert(id, cache);
        self.windows.insert(index, window);

        let before = self.title_key();
        for intent in intents {
            self.apply(intent, id, input, store);
        }
        if self.title_key() != before {
            self.retitle();
        }
        self.windows
            .iter()
            .find(|window| window.id == id)
            .map(|window| window.pixels.as_slice())
    }

    fn draw_window(
        &self,
        window: &mut super::Window,
        view: &super::ViewState,
        cache: &views::RenderCache,
        input: &Input,
        size: (usize, usize),
        intents: &mut Vec<super::Intent>,
    ) {
        let scale = self.scale_for(size.0);
        let kind = window.kind;
        let ui_input = to_ui(input, scale);
        let (width, height) = size;
        let ui_width = i32::try_from(width).unwrap_or(1) / scale;
        let ui_height = i32::try_from(height).unwrap_or(1) / scale;
        let roles = self.assets.theme.roles;
        let tabs = self.documents.tabs();
        window.ui.modal = window.dialog.is_some();
        window.canvas.reset(ui_width, ui_height, roles.window);
        if window.pixels.len() != width * height {
            window.pixels.clear();
            window.pixels.resize(width * height, 0);
        }
        window.size = size;

        let (canvas_area, preview) = {
            let mut ui = Ui::new(
                &mut window.canvas,
                &self.assets.theme,
                &self.assets.font,
                &self.assets.icons,
                &ui_input,
                &mut window.ui,
            );
            let mut context = Context {
                kind,
                raw: input,
                scale,
                document: self.document(),
                editing: &self.editing,
                view,
                popped: &window.popped,
                tabs: &tabs,
                active_tab: self.documents.active_index(),
                recent: self.recent.paths(),
                dialog: window.dialog.as_mut(),
                selected: window.selected.as_ref(),
                palette_edit: window.palette_edit,
                panels: &mut window.panels,
                code: &mut window.code,
                picker: window.picker,
                fields: &mut window.fields,
                renaming: window.renaming,
                cache,
                guide: self
                    .stroke
                    .as_ref()
                    .map_or(&[][..], |stroke| stroke.guide()),
                last_pointer: &mut window.last_pointer,
                hover: None,
                menu: views::MenuState {
                    theme: self.theme,
                    scale: self.scale,
                },
                help: window.help,
                preview: None,
                intents,
            };
            let area = views::draw(&mut ui, &mut context);
            ui.finish();
            (area, context.preview)
        };
        present(
            &window.canvas,
            &self.assets.theme.palette,
            scale,
            &mut window.pixels,
            width,
        );
        window.area = canvas_area.map(|area| {
            px_ui::rect(
                area.x * scale,
                area.y * scale,
                area.width * scale,
                area.height * scale,
            )
        });
        if let Some(area) = canvas_area {
            self.paint_canvas(window, area, (view, cache, input), scale);
        }
        if let Some(area) = preview {
            views::paint_preview(&mut window.pixels, width, area, scale, cache);
        }
        present_top(
            &window.canvas,
            &self.assets.theme.palette,
            scale,
            &mut window.pixels,
            width,
        );
    }

    fn paint_canvas(
        &self,
        window: &mut super::Window,
        area: px_ui::Rect,
        (view, cache, input): (&super::ViewState, &views::RenderCache, &Input),
        scale: i32,
    ) {
        let guide = self
            .stroke
            .as_ref()
            .map_or(&[][..], |stroke| stroke.guide());
        let canvas = views::CanvasPaint {
            document: self.document(),
            view,
            cache,
            editing: &self.editing,
            guide,
            input,
            theme: &self.assets.theme,
        };
        let width = window.size.0;
        views::paint_canvas(&mut window.pixels, width, area, scale, &canvas);
    }

    fn tick(&mut self, now: u64) {
        if !self.editing.playing {
            self.editing.played_at = now;
            self.editing.played_frames = 0;
            return;
        }
        let Some(scene) = self.document().and_then(crate::domain::Document::scene) else {
            return;
        };
        let timing = scene.timing();
        let fps = u64::from(timing.fps.get().max(1));
        let duration = timing.duration.get().max(1);
        let due = now.saturating_sub(self.editing.played_at) * fps / 1000;
        let behind = due.saturating_sub(self.editing.played_frames);
        if behind == 0 {
            return;
        }
        if behind > u64::from(duration) {
            self.editing.played_at = now;
            self.editing.played_frames = 0;
        } else {
            self.editing.played_frames = due;
        }
        let advance = u32::try_from(behind % u64::from(duration)).unwrap_or(0);
        for window in &mut self.windows {
            if window.kind.has_canvas() {
                window.view.frame = (window.view.frame + advance) % duration;
            }
        }
    }
}

fn to_ui(input: &Input, scale: i32) -> Input {
    let scale = scale.max(1);
    Input {
        pointer: input
            .pointer
            .map(|(x, y)| (x.div_euclid(scale), y.div_euclid(scale))),
        pointer_anywhere: (
            input.pointer_anywhere.0.div_euclid(scale),
            input.pointer_anywhere.1.div_euclid(scale),
        ),
        ..input.clone()
    }
}
