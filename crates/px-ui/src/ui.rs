use std::collections::BTreeMap;

use px_format::Font;

use crate::area::{Area, Rect};
use crate::bitmap::Icons;
use crate::canvas::UiCanvas;
use crate::input::Input;
use crate::theme::Theme;

/// A widget's identity across frames: a deterministic hash of names and numbers.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Id(u64);

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0100_0000_01b3;

impl Id {
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self(FNV_OFFSET).with_bytes(name.as_bytes())
    }

    #[must_use]
    pub fn with(self, value: u64) -> Self {
        self.with_bytes(&value.to_le_bytes())
    }

    #[must_use]
    pub fn with_name(self, name: &str) -> Self {
        self.with_bytes(name.as_bytes())
    }

    fn with_bytes(self, bytes: &[u8]) -> Self {
        Self(bytes.iter().fold(self.0, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
        }))
    }
}

/// What lasts between frames: the widget being pressed or dragged, keyboard focus,
/// scroll offsets and text cursors.
#[derive(Clone, Debug, Default)]
pub struct UiState {
    pub active: Option<Id>,
    pub focus: Option<Id>,
    /// Where the active widget was pressed.
    pub press: (i32, i32),
    pub scroll: BTreeMap<Id, i32>,
    pub cursor: BTreeMap<Id, usize>,
    /// The open menu of a menu bar.
    pub open_menu: Option<usize>,
    /// Popups drawn last frame: widgets under them don't see the pointer.
    pub blocked: Vec<Rect>,
    /// Popups drawn this frame, which block from the next one.
    pub blocking: Vec<Rect>,
    /// A dialog is open: only widgets drawn inside popups see the pointer.
    pub modal: bool,
    /// The last clicked widget and when, for double clicks.
    pub last_click: Option<(Id, u64)>,
    /// Text being typed into number fields, committed on Enter or focus loss.
    pub drafts: BTreeMap<Id, String>,
}

const DOUBLE_CLICK_MS: u64 = 400;

/// How the pointer met a widget this frame.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent facts about one frame"
)]
pub struct Response {
    pub hovered: bool,
    /// Pressed this frame.
    pub pressed: bool,
    /// Held since it was pressed on this widget.
    pub held: bool,
    /// Released over the widget it was pressed on.
    pub clicked: bool,
    /// Clicked within `DOUBLE_CLICK_MS` of a click on the same widget.
    pub double_clicked: bool,
    /// Released after being held, wherever the pointer is.
    pub released: bool,
    pub right_clicked: bool,
    /// Pointer movement since the press, while held.
    pub drag: (i32, i32),
}

/// One frame of UI for one window.
pub struct Ui<'a> {
    pub canvas: &'a mut UiCanvas,
    pub theme: &'a Theme,
    pub font: &'a Font,
    pub icons: &'a Icons,
    pub input: &'a Input,
    pub state: &'a mut UiState,
    tooltip: Option<(String, (i32, i32))>,
    /// Drawing a popup: on the top layer, and not blocked by popups.
    in_popup: bool,
}

impl<'a> Ui<'a> {
    pub fn new(
        canvas: &'a mut UiCanvas,
        theme: &'a Theme,
        font: &'a Font,
        icons: &'a Icons,
        input: &'a Input,
        state: &'a mut UiState,
    ) -> Self {
        state.blocked = core::mem::take(&mut state.blocking);
        Self {
            canvas,
            theme,
            font,
            icons,
            input,
            state,
            tooltip: None,
            in_popup: false,
        }
    }

    /// Draws `draw` as a popup covering `area`: on the top layer, above the document, and
    /// taking the pointer from widgets underneath from the next frame on.
    pub fn popup<T>(&mut self, area: Rect, draw: impl FnOnce(&mut Self) -> T) -> T {
        self.state.blocking.push(area);
        let was_on_top = self.canvas.set_on_top(true);
        let was_in_popup = core::mem::replace(&mut self.in_popup, true);
        let clip = self.canvas.clip_to(self.canvas.bounds());
        let result = draw(self);
        self.canvas.restore_clip(clip);
        self.in_popup = was_in_popup;
        self.canvas.set_on_top(was_on_top);
        result
    }

    /// Whether a dialog or a popup this widget is not part of takes the pointer.
    fn blocked(&self) -> bool {
        !self.in_popup
            && (self.state.modal
                || self.input.pointer.is_some_and(|(x, y)| {
                    self.state.blocked.iter().any(|area| area.contains(x, y))
                }))
    }

    /// Whether the pointer is over `area` and not over a popup covering it.
    #[must_use]
    pub fn hovering(&self, area: Rect) -> bool {
        !self.blocked()
            && self
                .input
                .pointer
                .is_some_and(|(x, y)| area.intersect(self.canvas.bounds()).contains(x, y))
    }

    /// Hit-tests `area` for widget `id` and tracks presses and drags.
    pub fn interact(&mut self, id: Id, area: Rect) -> Response {
        let input = self.input;
        let hovered = self.hovering(area) && self.state.active.is_none_or(|active| active == id);
        let pressed = hovered && input.pressed.left;
        if pressed {
            self.state.active = Some(id);
            self.state.press = input.pointer.unwrap_or_default();
        }
        let held = self.state.active == Some(id) && input.down.left;
        let released = self.state.active == Some(id) && input.released.left;
        if released {
            self.state.active = None;
        }
        let (px, py) = input.pointer_anywhere;
        let drag = if held || released {
            (px - self.state.press.0, py - self.state.press.1)
        } else {
            (0, 0)
        };
        let clicked = released && hovered;
        let double_clicked = clicked
            && self.state.last_click.is_some_and(|(last, at)| {
                last == id && input.time_ms.saturating_sub(at) <= DOUBLE_CLICK_MS
            });
        if clicked {
            self.state.last_click = Some((id, input.time_ms));
        }
        Response {
            hovered,
            pressed,
            held,
            clicked,
            double_clicked,
            released,
            right_clicked: hovered && input.pressed.right,
            drag,
        }
    }

    /// Shows `text` near the pointer at the end of the frame.
    pub fn tooltip(&mut self, text: &str) {
        if let Some(pointer) = self.input.pointer {
            self.tooltip = Some((text.to_owned(), pointer));
        }
    }

    /// Draws what must be on top of everything: the tooltip.
    pub fn finish(&mut self) {
        if let Some((text, (x, y))) = self.tooltip.take() {
            let was_on_top = self.canvas.set_on_top(true);
            crate::widgets::draw_tooltip(self, &text, x, y);
            self.canvas.set_on_top(was_on_top);
        }
    }
}
