use crate::area::{Area, Rect, rect};
use crate::input::Key;
use crate::lists::{SCROLLBAR, scroll_area};
use crate::text::{draw_text, text_width};
use crate::ui::{Id, Ui};
use crate::widgets::{ROW, engraved_text};

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
const ITEM: i32 = ROW + 2;
pub const MENU_KEY: Key = Key::Function(10);

impl MenuItem<'_> {
    fn is_separator(&self) -> bool {
        self.label.is_empty()
    }

    fn height(&self) -> i32 {
        if self.is_separator() { SEPARATOR } else { ITEM }
    }
}

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
            ui.state.menu_item = None;
        } else if response.hovered && ui.state.open_menu.is_some() && !open {
            ui.state.open_menu = Some(index);
            ui.state.menu_item = None;
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
    keyboard(ui, menus);
    let open = ui.state.open_menu?;
    let menu = menus.get(open)?;
    let title = titles[open];
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
        ui.state.menu_item = None;
    }
    chosen.map(|item| (open, item))
}

fn keyboard(ui: &mut Ui<'_>, menus: &[Menu<'_>]) {
    if menus.is_empty() {
        return;
    }
    let Some(open) = ui.state.open_menu else {
        if ui.take_key(MENU_KEY) {
            ui.state.open_menu = Some(0);
            ui.state.menu_item = next_item(&menus[0].items, None, false);
        }
        return;
    };
    if ui.take_key(Key::Escape) || ui.take_key(MENU_KEY) {
        ui.state.open_menu = None;
        ui.state.menu_item = None;
        return;
    }
    let count = menus.len();
    let moved = if ui.take_key(Key::Right) {
        Some((open + 1) % count)
    } else if ui.take_key(Key::Left) {
        Some((open + count - 1) % count)
    } else {
        None
    };
    if let Some(menu) = moved {
        ui.state.open_menu = Some(menu);
        ui.state.menu_item = next_item(&menus[menu].items, None, false);
        return;
    }
    let items = &menus[open.min(count - 1)].items;
    if ui.take_key(Key::Down) {
        ui.state.menu_item = next_item(items, ui.state.menu_item, false);
    } else if ui.take_key(Key::Up) {
        ui.state.menu_item = next_item(items, ui.state.menu_item, true);
    }
}

fn next_item(items: &[MenuItem<'_>], from: Option<usize>, backwards: bool) -> Option<usize> {
    let usable = |index: &usize| items[*index].enabled && !items[*index].is_separator();
    let count = items.len();
    let start = from.unwrap_or(if backwards {
        0
    } else {
        count.saturating_sub(1)
    });
    (1..=count)
        .map(|step| {
            if backwards {
                (start + count * step - step) % count
            } else {
                (start + step) % count
            }
        })
        .find(usable)
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
    let content: i32 = menu.items.iter().map(MenuItem::height).sum();
    let bounds = ui.canvas.bounds();
    let height = (content + 2)
        .min(bounds.bottom() - title.bottom())
        .max(ITEM + 2);
    let width = if content + 2 > height {
        width + SCROLLBAR
    } else {
        width
    };
    let left = title.x.min(bounds.right() - width).max(0);
    let area = rect(left, title.bottom(), width, height);
    ui.popup(area, |ui| {
        ui.canvas.fill(area, roles.panel);
        ui.canvas.frame(area, roles.ink_dim);
        let scroll_id = id.with_name("scroll");
        reveal_highlight(ui, scroll_id, menu, area.inset(1).height);
        let (view, offset) = scroll_area(ui, scroll_id, area.inset(1), content);
        let clip = ui.canvas.clip_to(view);
        let mut y = view.y - offset;
        let mut chosen = None;
        for (index, item) in menu.items.iter().enumerate() {
            let row = rect(view.x, y, view.width, item.height());
            y += item.height();
            if item.is_separator() {
                ui.canvas.fill(
                    rect(row.x + 1, row.y + SEPARATOR / 2, row.width - 2, 1),
                    roles.panel_dark,
                );
                continue;
            }
            let response = ui.interact(id.with(index as u64), row);
            if response.hovered && item.enabled {
                ui.state.menu_item = Some(index);
            }
            let highlighted = item.enabled && ui.state.menu_item == Some(index);
            if highlighted {
                ui.canvas.fill(row, roles.accent);
            }
            menu_row(ui, row, item, (check, highlighted));
            let pressed = highlighted && (ui.take_key(Key::Enter) || ui.take_key(Key::Space));
            if item.enabled && (response.clicked || pressed) {
                chosen = Some(index);
            }
        }
        ui.canvas.restore_clip(clip);
        chosen
    })
}

fn reveal_highlight(ui: &mut Ui<'_>, scroll_id: Id, menu: &Menu<'_>, visible: i32) {
    let Some(item) = ui.state.menu_item else {
        return;
    };
    let top: i32 = menu.items.iter().take(item).map(MenuItem::height).sum();
    let offset = ui.state.scroll.get(&scroll_id).copied().unwrap_or(0);
    let shown = offset.clamp(top + ITEM - visible, top);
    if shown != offset
        && ui
            .input
            .keys
            .iter()
            .any(|key| matches!(key, Key::Up | Key::Down))
    {
        ui.state.scroll.insert(scroll_id, shown);
    }
}

fn menu_row(ui: &mut Ui<'_>, row: Rect, item: &MenuItem<'_>, (check, highlighted): (i32, bool)) {
    let roles = ui.theme.roles;
    let text_y = row.y + (row.height - i32::from(ui.font.height())) / 2;
    let shortcut_x = row.right() - PAD - text_width(ui.font, item.shortcut);
    let mark = if item.enabled {
        roles.ink
    } else {
        roles.ink_disabled
    };
    if item.checked == Some(true) {
        ui.canvas.fill(rect(row.x + 3, text_y + 2, 3, 3), mark);
    }
    if !item.enabled {
        engraved_text(ui, row.x + check, text_y, item.label);
        engraved_text(ui, shortcut_x, text_y, item.shortcut);
        return;
    }
    let shortcut_ink = if highlighted {
        roles.ink
    } else {
        roles.ink_dim
    };
    draw_text(
        ui.canvas,
        ui.font,
        row.x + check,
        text_y,
        item.label,
        roles.ink,
    );
    draw_text(
        ui.canvas,
        ui.font,
        shortcut_x,
        text_y,
        item.shortcut,
        shortcut_ink,
    );
}
