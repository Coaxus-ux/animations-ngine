#![forbid(unsafe_code)]

mod area;
mod bitmap;
mod canvas;
mod choice;
mod color_picker;
mod dialog;
mod fields;
mod input;
mod lists;
mod menus;
mod number;
mod present;
mod tabs;
mod text;
mod theme;
mod ui;
mod widgets;

pub use area::{Area, Rect, rect};
pub use bitmap::{Bitmap, Icons};
pub use canvas::{Overlay, Pen, UiCanvas};
pub use choice::segmented;
pub use color_picker::{hue_bar, sv_square};
pub use dialog::{DialogAction, modal, ok_cancel};
pub use fields::{FieldResponse, slider, text_field};
pub use input::{Buttons, Input, Key, Modifiers};
pub use lists::{drop_index, list_row, list_row_icon, scroll_area};
pub use menus::{MENU_KEY, Menu, MenuItem, menu_bar};
pub use number::number_field;
pub use present::{fill, pack, present, present_top};
pub use tabs::{Tab, TabAction, tab_strip};
pub use text::{draw_text, fit_text, text_width};
pub use theme::{Roles, Theme};
pub use ui::{Id, Response, Ui, UiState};
pub use widgets::{
    HEADER, ICON, ROW, SMALL_ICON, TOOL_BUTTON, button, button_if, centered_label, checkbox,
    header_button, icon_button, label, panel, separator, toggle_icon,
};
