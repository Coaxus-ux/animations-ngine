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
use px_ui::{Area, HEADER, Id, Input, Rect, Ui, centered_label, header_button, panel};

pub use canvas::{CanvasPaint, paint_canvas};
pub use doc_view::{Placement, RenderCache};
pub use layout::tab_strip_area;
pub use menus::MenuState;

use crate::app::{
    CodeBuffer, Dialog, Editing, Intent, PanelKind, Picker, TabInfo, ViewState, WindowKind,
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
    pub intents: &'a mut Vec<Intent>,
}

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
