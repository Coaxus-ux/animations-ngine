//! Panels drawn with px-ui. Views read the document and the editing state and turn input
//! into `Intent`s; they never change the document themselves (D-015).

mod canvas;
mod code;
mod doc_view;
mod help;
mod inspector;
mod layers;
mod menus;
mod palette;
mod timeline;
mod tools;

use std::collections::{BTreeMap, BTreeSet};

use px_ui::{Area, HEADER, Id, Input, Rect, Ui, header_button, panel};

pub use canvas::{CanvasPaint, paint_canvas};
pub use doc_view::{Placement, RenderCache};
pub use menus::MenuState;

use crate::app::{CodeBuffer, CodeMode, Editing, Intent, PanelKind, Picker, ViewState, WindowKind};
use crate::domain::Document;

/// Side panel width: a fifth of the window, within these bounds (UI pixels).
const SIDE: (i32, i32) = (104, 150);
const TIMELINE: i32 = 70;
/// Below this width (UI pixels) the inspector folds away (Window menu brings it back).
const ROOM_FOR_INSPECTOR: i32 = 600;

/// Everything a window's views read, and where they put their intents.
pub struct Context<'a> {
    pub kind: WindowKind,
    /// The frame's input in window pixels (the canvas works at that precision).
    pub raw: &'a Input,
    pub scale: i32,
    pub document: Option<&'a Document>,
    pub editing: &'a Editing,
    /// This window's view, or for a torn-off panel, the canvas window it acts on.
    pub view: &'a ViewState,
    pub popped: &'a BTreeSet<PanelKind>,
    pub code: &'a mut CodeBuffer,
    pub picker: Option<Picker>,
    pub fields: &'a mut BTreeMap<Id, String>,
    pub renaming: Option<usize>,
    pub last_click: &'a mut Option<(Id, u64)>,
    pub cache: &'a RenderCache,
    pub guide: &'a [(i32, i32)],
    /// Where the pointer was last frame, in window pixels (for panning).
    pub last_pointer: &'a mut Option<(i32, i32)>,
    /// The canvas pixel under the pointer, found by the canvas view for the status bar.
    pub hover: Option<(i32, i32)>,
    pub menu: MenuState,
    pub help: bool,
    pub intents: &'a mut Vec<Intent>,
}

/// Whether this click on widget `id` follows another on it within 400 ms.
fn double_click(context: &mut Context<'_>, id: Id, now: u64) -> bool {
    let double = context
        .last_click
        .is_some_and(|(last, at)| last == id && now.saturating_sub(at) < 400);
    *context.last_click = Some((id, now));
    double
}

/// Draws the window; returns where the canvas goes (UI pixels), drawn after the UI.
pub fn draw(ui: &mut Ui<'_>, context: &mut Context<'_>) -> Option<Rect> {
    let bounds = ui.canvas.bounds();
    let area = match context.kind {
        WindowKind::Panel(kind) => {
            panel_view(ui, context, kind, bounds);
            None
        }
        WindowKind::Canvas | WindowKind::Main => editor(ui, context, bounds),
    };
    if context.help {
        help::help(ui, context);
    }
    area
}

/// Menus on top, the tool's options under them, tools down the left, panels around the
/// canvas and the status bar at the bottom.
fn editor(ui: &mut Ui<'_>, context: &mut Context<'_>, bounds: Rect) -> Option<Rect> {
    let (menu_area, rest) = bounds.take_top(px_ui::ROW + 3);
    let (options, rest) = rest.take_top(px_ui::ROW + 5);
    let (status, rest) = rest.take_bottom(px_ui::ROW + 2);
    let (column, rest) = rest.take_left(px_ui::TOOL_BUTTON + 6);
    tools::options(ui, context, options);
    tools::column(ui, context, column);
    let area = if context.kind == WindowKind::Main {
        main_panels(ui, context, rest)
    } else {
        canvas::canvas(ui, context, rest)
    };
    tools::status(ui, context, status);
    menus::menus(ui, context, menu_area);
    area
}

fn main_panels(ui: &mut Ui<'_>, context: &mut Context<'_>, mut rest: Rect) -> Option<Rect> {
    let docked = |panel: PanelKind| !context.popped.contains(&panel);
    let side = (ui.canvas.width() / 5).clamp(SIDE.0, SIDE.1);
    if docked(PanelKind::Layers) || docked(PanelKind::Palette) {
        let (left, remaining) = rest.take_left(side);
        rest = remaining;
        match (docked(PanelKind::Layers), docked(PanelKind::Palette)) {
            (true, true) => {
                let (top, bottom) = left.take_top(left.height / 2);
                panel_view(ui, context, PanelKind::Layers, top);
                panel_view(ui, context, PanelKind::Palette, bottom);
            }
            (true, false) => panel_view(ui, context, PanelKind::Layers, left),
            _ => panel_view(ui, context, PanelKind::Palette, left),
        }
    }
    if docked(PanelKind::Inspector) && ui.canvas.width() >= ROOM_FOR_INSPECTOR {
        let (right, remaining) = rest.take_right(side);
        rest = remaining;
        panel_view(ui, context, PanelKind::Inspector, right);
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

/// A panel with its title bar: a button tears it off into its own window (or docks it
/// back), and dragging the title bar out of the window tears it off too.
fn panel_view(ui: &mut Ui<'_>, context: &mut Context<'_>, kind: PanelKind, area: Rect) {
    let (header, body) = panel(ui, area, kind.title());
    let id = Id::new("panel").with_name(kind.title());
    let torn = matches!(context.kind, WindowKind::Panel(_));
    let (icon, tip) = if torn {
        ("popout", "Dock back into the main window")
    } else {
        ("popout", "Open in its own window (or drag the title out)")
    };
    if header_button(ui, id.with_name("pop"), header, 0, icon, tip) {
        context.intents.push(if torn {
            Intent::Dock(kind)
        } else {
            Intent::PopOut(kind)
        });
    }
    let drag_area = px_ui::rect(header.x, header.y, header.width - HEADER, header.height);
    let response = ui.interact(id.with_name("title"), drag_area);
    let (px, py) = ui.input.pointer_anywhere;
    if !torn && response.released && !ui.canvas.bounds().contains(px, py) {
        context.intents.push(Intent::PopOut(kind));
    }
    match kind {
        PanelKind::Layers => layers::layers(ui, context, body),
        PanelKind::Palette => palette::palette(ui, context, body),
        PanelKind::Timeline => timeline::timeline(ui, context, body),
        PanelKind::Inspector => inspector::inspector(ui, context, body),
        PanelKind::Code => code::code(ui, context, body),
    }
}
