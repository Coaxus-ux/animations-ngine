//! Panels drawn with px-ui. Views read the document and the editing state and turn input
//! into `Intent`s; they never change the document themselves (D-015).

mod canvas;
mod code;
mod dialogs;
mod doc_view;
mod elements;
mod help;
mod home;
mod inspector;
mod layers;
mod layout;
mod menus;
mod menus_file;
mod menus_palette;
mod palette;
mod palette_edit;
mod timeline;
mod tools;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use px_format::edit::NodePath;
use px_ui::{Area, HEADER, Id, Input, Rect, Ui, header_button, panel};

pub use canvas::{CanvasPaint, paint_canvas};
pub use doc_view::{Placement, RenderCache};
pub use layout::tab_strip_area;
pub use menus::MenuState;

use crate::app::{
    CodeBuffer, Dialog, Editing, Intent, PanelKind, Picker, TabInfo, ViewState, WindowKind,
};
use crate::domain::Document;

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
    /// Open documents in tab order and the active one.
    pub tabs: &'a [TabInfo],
    pub active_tab: Option<usize>,
    /// Files opened before, latest first (the File menu lists them).
    pub recent: &'a [PathBuf],
    pub dialog: Option<&'a mut Dialog>,
    pub selected: Option<&'a NodePath>,
    pub palette_edit: bool,
    pub code: &'a mut CodeBuffer,
    pub picker: Option<Picker>,
    pub fields: &'a mut BTreeMap<Id, String>,
    pub renaming: Option<usize>,
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

/// Draws the window; returns where the canvas goes (UI pixels), drawn after the UI.
pub fn draw(ui: &mut Ui<'_>, context: &mut Context<'_>) -> Option<Rect> {
    let bounds = ui.canvas.bounds();
    let area = match context.kind {
        WindowKind::Panel(kind) => {
            panel_view(ui, context, kind, bounds);
            None
        }
        WindowKind::Canvas | WindowKind::Main => layout::editor(ui, context, bounds),
    };
    if context.help {
        help::help(ui, context);
    }
    if context.dialog.is_some() {
        dialogs::draw(ui, context);
    }
    area
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
        PanelKind::Palette if context.palette_edit => palette_edit::palette_edit(ui, context, body),
        PanelKind::Palette => palette::palette(ui, context, body),
        PanelKind::Elements => elements::elements(ui, context, body),
        PanelKind::Timeline => timeline::timeline(ui, context, body),
        PanelKind::Inspector => inspector::inspector(ui, context, body),
        PanelKind::Code => code::code(ui, context, body),
    }
}
