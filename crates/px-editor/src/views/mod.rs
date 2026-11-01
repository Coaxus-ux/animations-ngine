mod canvas;
mod code;
mod columns;
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
mod panel_frame;
mod preview;
mod timeline;
mod tools;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use px_format::edit::NodePath;
use px_ui::{Id, Input, Rect, Ui, centered_label};

pub use canvas::{CanvasPaint, paint_canvas};
pub use doc_view::{Placement, RenderCache};
pub use layout::tab_strip_area;
pub use menus::MenuState;
pub use preview::paint_preview;

use crate::app::{
    CodeBuffer, Dialog, Editing, Intent, PanelKind, Panels, Picker, TabInfo, ViewState, WindowKind,
};
use crate::domain::Document;

pub struct Context<'a> {
    pub kind: WindowKind,
    pub raw: &'a Input,
    pub scale: i32,
    pub document: Option<&'a Document>,
    pub editing: &'a Editing,
    pub view: &'a ViewState,
    pub popped: &'a BTreeSet<PanelKind>,
    pub tabs: &'a [TabInfo],
    pub active_tab: Option<usize>,
    pub recent: &'a [PathBuf],
    pub dialog: Option<&'a mut Dialog>,
    pub selected: Option<&'a NodePath>,
    pub palette_edit: bool,
    pub panels: &'a mut Panels,
    pub code: &'a mut CodeBuffer,
    pub picker: Option<Picker>,
    pub fields: &'a mut BTreeMap<Id, String>,
    pub renaming: Option<usize>,
    pub cache: &'a RenderCache,
    pub guide: &'a [(i32, i32)],
    pub last_pointer: &'a mut Option<(i32, i32)>,
    pub hover: Option<(i32, i32)>,
    pub menu: MenuState,
    pub help: bool,
    pub preview: Option<Rect>,
    pub intents: &'a mut Vec<Intent>,
}

pub fn draw(ui: &mut Ui<'_>, context: &mut Context<'_>) -> Option<Rect> {
    let bounds = ui.canvas.bounds();
    let area = match context.kind {
        WindowKind::Panel(kind) => {
            panel_frame::panel_view(ui, context, kind, bounds);
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

fn unavailable(ui: &mut Ui<'_>, context: &Context<'_>, area: Rect) {
    let roles = ui.theme.roles;
    let (text, ink) = match context.document {
        None => ("No scene is open", roles.ink_dim),
        Some(_) if context.kind == WindowKind::Main => (
            "The scene doesn't load: see the code view (F2)",
            roles.warning,
        ),
        Some(_) => (
            "The scene doesn't load: fix it in the main window's code view",
            roles.warning,
        ),
    };
    centered_label(ui, area, text, ink);
}
