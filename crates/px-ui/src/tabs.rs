use crate::area::{Area, Rect, rect};
use crate::text::text_width;
use crate::ui::{Id, Ui};
use crate::widgets::{SMALL_ICON, centered_label, label};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Tab {
    pub label: String,
    pub dirty: bool,
    pub closable: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TabAction {
    #[default]
    None,
    Select(usize),
    Close(usize),
    Home,
}

const PAD: i32 = 4;
const CLOSE_BOX: i32 = SMALL_ICON + 2;

/// A row of tabs after a home tab (active while `active` is `None`). Each tab shows its
/// label (with `*` while dirty) and, while hovered, a close box when closable; the middle
/// button closes too. Tabs shrink to share `area` when they don't fit.
pub fn tab_strip(
    ui: &mut Ui<'_>,
    id: Id,
    area: Rect,
    tabs: &[Tab],
    active: Option<usize>,
) -> TabAction {
    let roles = ui.theme.roles;
    ui.canvas.fill(area, roles.panel_dark);
    let (home, rest) = area.take_left(area.height + PAD);
    let mut action = TabAction::None;
    let home_response = ui.interact(id.with_name("home"), home);
    tab_face(ui, home, active.is_none(), home_response.hovered);
    draw_icon(ui, home, "home", "H", active.is_none());
    if home_response.clicked {
        action = TabAction::Home;
    }
    let widths = tab_widths(ui, tabs, rest.width);
    let mut x = rest.x;
    for (index, (tab, width)) in tabs.iter().zip(widths).enumerate() {
        let cell = rect(x, rest.y, width, rest.height);
        x += width;
        let is_active = active == Some(index);
        let hovered = ui.hovering(cell);
        let close = rect(
            cell.right() - CLOSE_BOX - 1,
            cell.y + (cell.height - CLOSE_BOX) / 2,
            CLOSE_BOX,
            CLOSE_BOX,
        );
        let show_close = tab.closable && hovered;
        let close_clicked = show_close
            && ui
                .interact(id.with(index as u64).with_name("close"), close)
                .clicked;
        let response = ui.interact(id.with(index as u64), cell);
        tab_face(ui, cell, is_active, response.hovered);
        let text_right = if show_close { close.x } else { cell.right() };
        let text = if tab.dirty {
            format!("{}*", tab.label)
        } else {
            tab.label.clone()
        };
        let ink = if is_active { roles.ink } else { roles.ink_dim };
        let text_area = rect(
            cell.x + PAD,
            cell.y,
            text_right - cell.x - 2 * PAD,
            cell.height,
        );
        label(ui, text_area, &text, ink);
        if show_close {
            draw_icon(ui, close, "close", "x", is_active);
        }
        if close_clicked || (response.hovered && ui.input.pressed.middle) {
            action = TabAction::Close(index);
        } else if response.clicked {
            action = TabAction::Select(index);
        }
    }
    action
}

fn tab_widths(ui: &Ui<'_>, tabs: &[Tab], available: i32) -> Vec<i32> {
    let natural: Vec<i32> = tabs
        .iter()
        .map(|tab| text_width(ui.font, &tab.label) + i32::from(tab.dirty) * 4 + 2 * PAD + CLOSE_BOX)
        .collect();
    let total: i32 = natural.iter().sum();
    if total <= available || tabs.is_empty() {
        return natural;
    }
    let count = i32::try_from(tabs.len()).unwrap_or(1).max(1);
    let share = available / count;
    natural.iter().map(|_| share.max(0)).collect()
}

fn tab_face(ui: &mut Ui<'_>, cell: Rect, active: bool, hovered: bool) {
    let roles = ui.theme.roles;
    let face = rect(cell.x, cell.y + 1, (cell.width - 1).max(0), cell.height - 1);
    ui.canvas.fill(
        face,
        if active {
            roles.panel
        } else {
            roles.tab_inactive
        },
    );
    if hovered && !active {
        ui.canvas.dither(face.inset(1), roles.panel_light);
    }
    if active {
        ui.canvas
            .fill(rect(face.x, face.y, face.width, 1), roles.accent);
    }
}

fn draw_icon(ui: &mut Ui<'_>, area: Rect, icon: &str, fallback: &str, bright: bool) {
    let roles = ui.theme.roles;
    let tint = (!bright).then_some(roles.ink_dim);
    if let Some(bitmap) = ui.icons.get(icon) {
        let x = area.x + (area.width - bitmap.width()) / 2;
        let y = area.y + (area.height - bitmap.height()) / 2;
        ui.canvas.blit(bitmap, x, y, tint);
    } else {
        centered_label(ui, area, fallback, tint.unwrap_or(roles.ink));
    }
}
