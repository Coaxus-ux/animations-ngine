use crate::area::{Area, Rect, rect};
use crate::input::Key;
use crate::text::{draw_text, text_width};
use crate::ui::{Id, Ui};
use crate::widgets::ROW;

#[derive(Clone, Copy, Debug)]
pub struct MenuItem<'a> {
    pub label: &'a str,
    pub shortcut: &'a str,
    pub enabled: bool,
    pub checked: Option<bool>,
}

impl<'a> MenuItem<'a> {
    #[must_use]
    pub const fn new(label: &'a str, shortcut: &'a str) -> Self {
        Self {
            label,
            shortcut,
            enabled: true,
            checked: None,
        }
    }

    #[must_use]
    pub const fn separator() -> Self {
        Self::new("", "")
    }

    #[must_use]
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[must_use]
    pub const fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }
}

pub struct Menu<'a> {
    pub title: &'a str,
    pub items: Vec<MenuItem<'a>>,
}

const PAD: i32 = 6;
const SEPARATOR: i32 = 5;

pub fn menu_bar(ui: &mut Ui<'_>, area: Rect, menus: &[Menu<'_>]) -> Option<(usize, usize)> {
    let roles = ui.theme.roles;
    let id = Id::new("menu-bar");
    ui.canvas.fill(area, roles.header);
    let mut x = area.x + 2;
    let mut titles = Vec::with_capacity(menus.len());
    for (index, menu) in menus.iter().enumerate() {
        let width = text_width(ui.font, menu.title) + 2 * PAD;
        let title = rect(x, area.y, width, area.height);
        x += width;
        let response = ui.interact(id.with(index as u64), title);
        let open = ui.state.open_menu == Some(index);
        if response.pressed {
            ui.state.open_menu = if open { None } else { Some(index) };
        } else if response.hovered && ui.state.open_menu.is_some() {
            ui.state.open_menu = Some(index);
        }
        if ui.state.open_menu == Some(index) {
            ui.canvas.fill(title, roles.accent);
        } else if response.hovered {
            ui.canvas.dither(title, roles.panel_light);
        }
        let y = title.y + (title.height - i32::from(ui.font.height())) / 2;
        draw_text(ui.canvas, ui.font, title.x + PAD, y, menu.title, roles.ink);
        titles.push(title);
    }
    let open = ui.state.open_menu?;
    let menu = menus.get(open)?;
    let title = titles[open];
    if ui.input.key(Key::Escape) {
        ui.state.open_menu = None;
        return None;
    }
    let chosen = dropdown(ui, id.with(open as u64), title, menu);
    let clicked_away = ui.input.pressed.left
        && ui
            .input
            .pointer
            .is_some_and(|(px, py)| !area.contains(px, py))
        && !ui.state.blocked.iter().any(|blocked| {
            ui.input
                .pointer
                .is_some_and(|(px, py)| blocked.contains(px, py))
        });
    if chosen.is_some() || clicked_away {
        ui.state.open_menu = None;
    }
    chosen.map(|item| (open, item))
}

fn dropdown(ui: &mut Ui<'_>, id: Id, title: Rect, menu: &Menu<'_>) -> Option<usize> {
    let roles = ui.theme.roles;
    let check = 10;
    let width = menu
        .items
        .iter()
        .map(|item| {
            text_width(ui.font, item.label) + text_width(ui.font, item.shortcut) + 3 * PAD + check
        })
        .max()
        .unwrap_or(0)
        .max(title.width);
    let height: i32 = menu
        .items
        .iter()
        .map(|item| {
            if item.label.is_empty() {
                SEPARATOR
            } else {
                ROW + 2
            }
        })
        .sum::<i32>()
        + 2;
    let bounds = ui.canvas.bounds();
    let left = title.x.min(bounds.right() - width).max(0);
    let area = rect(left, title.bottom(), width, height);
    ui.popup(area, |ui| {
        ui.canvas.fill(area, roles.panel);
        ui.canvas.frame(area, roles.ink_dim);
        let mut y = area.y + 1;
        let mut chosen = None;
        for (index, item) in menu.items.iter().enumerate() {
            if item.label.is_empty() {
                ui.canvas.fill(
                    rect(area.x + 2, y + SEPARATOR / 2, area.width - 4, 1),
                    roles.panel_dark,
                );
                y += SEPARATOR;
                continue;
            }
            let row = rect(area.x + 1, y, area.width - 2, ROW + 2);
            y += ROW + 2;
            let response = ui.interact(id.with(index as u64), row);
            if response.hovered && item.enabled {
                ui.canvas.fill(row, roles.accent);
            }
            let ink = if item.enabled {
                roles.ink
            } else {
                roles.ink_dim
            };
            let text_y = row.y + (row.height - i32::from(ui.font.height())) / 2;
            if item.checked == Some(true) {
                ui.canvas.fill(rect(row.x + 3, text_y + 2, 3, 3), ink);
            }
            draw_text(ui.canvas, ui.font, row.x + check, text_y, item.label, ink);
            let shortcut_x = row.right() - PAD - text_width(ui.font, item.shortcut);
            draw_text(
                ui.canvas,
                ui.font,
                shortcut_x,
                text_y,
                item.shortcut,
                roles.ink_dim,
            );
            if response.clicked && item.enabled {
                chosen = Some(index);
            }
        }
        chosen
    })
}
