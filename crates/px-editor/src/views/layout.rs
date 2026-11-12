use px_ui::{Area, Id, ROW, Rect, TOOL_BUTTON, Tab, TabAction, Ui, tab_strip};

use super::columns::{self, GRIP, column};
use super::panel_frame::{COLLAPSED, is_collapsed, panel_view};
use super::{Context, canvas, home, menus, tools};
use crate::app::{CodeMode, Intent, MIN_COLUMN, PanelKind, Side, WindowKind};

const SIDE: (i32, i32) = (104, 150);
const TIMELINE: i32 = 96;
const ROOM_FOR_INSPECTOR: i32 = 600;
const MENU_BAR: i32 = ROW + 3;
const TAB_STRIP: i32 = ROW + 3;
const CONTEXT_BAR: i32 = ROW + 5;
const STATUS_BAR: i32 = ROW + 2;
const TOOL_BAR: i32 = TOOL_BUTTON + 6;

#[must_use]
pub fn tab_strip_area(bounds: Rect) -> Rect {
    bounds.take_top(MENU_BAR).1.take_top(TAB_STRIP).0
}

pub(super) fn editor(ui: &mut Ui<'_>, context: &mut Context<'_>, bounds: Rect) -> Option<Rect> {
    let (menu_area, rest) = bounds.take_top(MENU_BAR);
    let (status, rest) = rest.take_bottom(STATUS_BAR);
    let area = match (context.kind, context.document.is_some()) {
        (WindowKind::Main, false) => {
            let (tabs, rest) = rest.take_top(TAB_STRIP);
            tab_bar(ui, context, tabs);
            home::home(ui, context, rest);
            None
        }
        (WindowKind::Main, true) => {
            let (tabs, rest) = rest.take_top(TAB_STRIP);
            let (options, rest) = rest.take_top(CONTEXT_BAR);
            tab_bar(ui, context, tabs);
            tools::options(ui, context, options);
            main_panels(ui, context, rest)
        }
        _ => {
            let (options, rest) = rest.take_top(CONTEXT_BAR);
            let (tool_bar, rest) = rest.take_right(TOOL_BAR);
            tools::options(ui, context, options);
            tools::column(ui, context, tool_bar);
            canvas::canvas(ui, context, rest)
        }
    };
    tools::status(ui, context, status, area);
    menus::menus(ui, context, menu_area);
    area
}

fn tab_bar(ui: &mut Ui<'_>, context: &mut Context<'_>, area: Rect) {
    let tabs: Vec<Tab> = context
        .tabs
        .iter()
        .map(|tab| Tab {
            label: tab.title.clone(),
            dirty: tab.dirty,
            closable: true,
        })
        .collect();
    let intent = match tab_strip(ui, Id::new("tabs"), area, &tabs, context.active_tab) {
        TabAction::None => return,
        TabAction::Select(index) => Intent::ActivateTab(index),
        TabAction::Close(index) => Intent::CloseTab {
            index,
            discard: false,
        },
        TabAction::Home => Intent::GoHome,
    };
    context.intents.push(intent);
}

fn main_panels(ui: &mut Ui<'_>, context: &mut Context<'_>, mut rest: Rect) -> Option<Rect> {
    let docked = |panel: PanelKind| !context.popped.contains(&panel);
    let auto = (ui.canvas.width() / 5).clamp(SIDE.0, SIDE.1);
    let widest = (ui.canvas.width() / 3).max(MIN_COLUMN);
    let width = |side: Side| {
        context
            .panels
            .width(side)
            .unwrap_or(auto)
            .clamp(MIN_COLUMN, widest)
    };
    let (left_width, right_width) = (width(Side::Left), width(Side::Right));
    let palette_right = context.panels.palette_right;
    if docked(PanelKind::Palette) && !palette_right {
        let (left, remaining) = rest.take_left(left_width);
        let (grip, remaining) = remaining.take_left(GRIP);
        rest = remaining;
        column(ui, context, &[PanelKind::Palette], left);
        columns::grip(ui, context, Side::Left, (left, grip));
    }
    let (tool_bar, remaining) = rest.take_right(TOOL_BAR);
    rest = remaining;
    tools::column(ui, context, tool_bar);
    let wide = ui.canvas.width() >= ROOM_FOR_INSPECTOR;
    let stacked: Vec<PanelKind> = [
        PanelKind::Palette,
        PanelKind::Layers,
        PanelKind::Elements,
        PanelKind::Inspector,
    ]
    .into_iter()
    .filter(|panel| docked(*panel))
    .filter(|panel| *panel != PanelKind::Palette || palette_right)
    .filter(|panel| *panel != PanelKind::Inspector || wide)
    .collect();
    if !stacked.is_empty() {
        let (right, remaining) = rest.take_right(right_width);
        let (grip, remaining) = remaining.take_right(GRIP);
        rest = remaining;
        column(ui, context, &stacked, right);
        columns::grip(ui, context, Side::Right, (right, grip));
    }
    if docked(PanelKind::Timeline) {
        let height = if is_collapsed(context, PanelKind::Timeline) {
            COLLAPSED
        } else {
            TIMELINE
        };
        let (bottom, remaining) = rest.take_bottom(height);
        rest = remaining;
        panel_view(ui, context, PanelKind::Timeline, bottom);
    }
    match (context.view.code, docked(PanelKind::Code)) {
        (CodeMode::Only, true) => {
            panel_view(ui, context, PanelKind::Code, rest);
            None
        }
        (CodeMode::Split, true) => {
            let (left, right) = rest.take_left(rest.width / 2);
            panel_view(ui, context, PanelKind::Code, right);
            canvas::canvas(ui, context, left)
        }
        _ => canvas::canvas(ui, context, rest),
    }
}
