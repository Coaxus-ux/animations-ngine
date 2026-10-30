//! A pixel-art UI toolkit whose every pixel is drawn by the engine: an indexed UI canvas in
//! a theme palette, the engine's pixel font, icons rendered from KDL scenes, and
//! immediate-mode widgets. No window library and no I/O (D-012, D-015). Memory profile:
//! allocation-conscious; canvases and state are reused across frames.
#![forbid(unsafe_code)]

mod area;
mod bitmap;
mod canvas;
mod fields;
mod input;
mod lists;
mod menus;
mod present;
mod text;
mod theme;
mod ui;
mod widgets;

pub use area::{Area, Rect, rect};
pub use bitmap::{Bitmap, Icons};
pub use canvas::{Pen, UiCanvas};
pub use fields::{FieldResponse, slider, text_field};
pub use input::{Buttons, Input, Key, Modifiers};
pub use lists::{drop_index, list_row, scroll_area};
pub use menus::{Menu, MenuItem, menu_bar};
pub use present::{fill, pack, present, present_top};
pub use text::{draw_text, fit_text, text_width};
pub use theme::{Roles, Theme};
pub use ui::{Id, Response, Ui, UiState};
pub use widgets::{
    HEADER, ICON, ROW, SMALL_ICON, TOOL_BUTTON, button, centered_label, checkbox, header_button,
    icon_button, label, panel, separator, toggle_icon,
};
