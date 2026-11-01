//! Where everything goes in a window, arranged like Aseprite: menus, tabs and the context
//! bar on top, the color bar down the left, tools and panels down the right, the timeline
//! and the status bar at the bottom, the canvas (or the code) in the middle.

use px_ui::{Area, Id, ROW, Rect, TOOL_BUTTON, Tab, TabAction, Ui, tab_strip};

use super::{Context, canvas, home, menus, panel_view, tools};
use crate::app::{CodeMode, Intent, PanelKind, WindowKind};

/// Side panel width: a fifth of the window, within these bounds (UI pixels).
const SIDE: (i32, i32) = (104, 150);
const TIMELINE: i32 = 70;
/// Below this width (UI pixels) the inspector folds away (Window menu brings it back).
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

/// Draws a canvas or main window; returns where the canvas goes (UI pixels).
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

/// The home tab and one tab per open document.
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
    let side = (ui.canvas.width() / 5).clamp(SIDE.0, SIDE.1);
    if docked(PanelKind::Palette) {
        let (left, remaining) = rest.take_left(side);
        rest = remaining;
        let (paints, palette) = left.take_bottom(tools::PAINTS);
        panel_view(ui, context, PanelKind::Palette, palette);
        tools::paints(ui, context, paints);
    }
    let (tool_bar, remaining) = rest.take_right(TOOL_BAR);
    rest = remaining;
    tools::column(ui, context, tool_bar);
    let wide = ui.canvas.width() >= ROOM_FOR_INSPECTOR;
    let stacked: Vec<PanelKind> = [PanelKind::Layers, PanelKind::Elements, PanelKind::Inspector]
        .into_iter()
        .filter(|panel| docked(*panel) && (*panel != PanelKind::Inspector || wide))
        .collect();
    if !stacked.is_empty() {
        let (right, remaining) = rest.take_right(side);
        rest = remaining;
        stack(ui, context, &stacked, right);
    }
    if docked(PanelKind::Timeline) {
        let (bottom, remaining) = rest.take_bottom(TIMELINE);
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

/// Panels one under another, sharing the height equally.
fn stack(ui: &mut Ui<'_>, context: &mut Context<'_>, panels: &[PanelKind], mut area: Rect) {
    let count = i32::try_from(panels.len()).unwrap_or(1).max(1);
    let each = area.height / count;
    for (index, panel) in panels.iter().enumerate() {
        let slot = if index + 1 == panels.len() {
            area
        } else {
            let (top, remaining) = area.take_top(each);
            area = remaining;
            top
        };
        panel_view(ui, context, *panel, slot);
    }
}
