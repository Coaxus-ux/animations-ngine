use std::collections::BTreeMap;

use px_format::Font;

use crate::area::{Area, Rect};
use crate::bitmap::Icons;
use crate::canvas::UiCanvas;
use crate::input::{Input, Key};
use crate::theme::Theme;

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

#[derive(Clone, Debug, Default)]
pub struct UiState {
    pub active: Option<Id>,
    pub focus: Option<Id>,
    pub press: (i32, i32),
    pub scroll: BTreeMap<Id, i32>,
    pub cursor: BTreeMap<Id, usize>,
    pub open_menu: Option<usize>,
    pub blocked: Vec<Rect>,
    pub blocking: Vec<Rect>,
    pub modal: bool,
    pub last_click: Option<(Id, u64)>,
    pub drafts: BTreeMap<Id, String>,
    pub key_focus: Option<Id>,
    pub menu_item: Option<usize>,
    pub(crate) wheel_rest: BTreeMap<Id, (f32, f32)>,
    stops: Vec<Stop>,
    last_stops: Vec<Stop>,
    used_keys: Vec<Key>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Stop {
    id: Id,
    takes_text: bool,
}

impl UiState {
    fn step_focus(&mut self, backwards: bool) {
        let ring = &self.last_stops;
        if ring.is_empty() {
            return;
        }
        let current = self
            .key_focus
            .and_then(|focus| ring.iter().position(|stop| stop.id == focus));
        let next = match current {
            None if backwards => ring.len() - 1,
            None => 0,
            Some(index) if backwards => (index + ring.len() - 1) % ring.len(),
            Some(index) => (index + 1) % ring.len(),
        };
        let stop = ring[next];
        self.key_focus = Some(stop.id);
        self.focus = stop.takes_text.then_some(stop.id);
    }

    fn tab_moves_focus(&self) -> bool {
        self.open_menu.is_none()
            && self
                .focus
                .is_none_or(|focus| self.last_stops.iter().any(|stop| stop.id == focus))
    }
}

const DOUBLE_CLICK_MS: u64 = 400;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent facts about one frame"
)]
pub struct Response {
    pub hovered: bool,
    pub pressed: bool,
    pub held: bool,
    pub clicked: bool,
    pub double_clicked: bool,
    pub released: bool,
    pub right_clicked: bool,
    pub drag: (i32, i32),
}

pub struct Ui<'a> {
    pub canvas: &'a mut UiCanvas,
    pub theme: &'a Theme,
    pub font: &'a Font,
    pub icons: &'a Icons,
    pub input: &'a Input,
    pub state: &'a mut UiState,
    tooltip: Option<(String, (i32, i32))>,
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
        state.last_stops = core::mem::take(&mut state.stops);
        state.used_keys.clear();
        if input.pressed.any() {
            state.key_focus = None;
        }
        if input.key(Key::Escape) && state.focus.is_none() {
            state.key_focus = None;
        }
        if input.key(Key::Tab) && !input.modifiers.command && state.tab_moves_focus() {
            state.step_focus(input.modifiers.shift);
        }
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

    fn blocked(&self) -> bool {
        !self.in_popup
            && (self.state.modal
                || self.input.pointer.is_some_and(|(x, y)| {
                    self.state.blocked.iter().any(|area| area.contains(x, y))
                }))
    }

    #[must_use]
    pub fn hovering(&self, area: Rect) -> bool {
        !self.blocked()
            && self
                .input
                .pointer
                .is_some_and(|(x, y)| area.intersect(self.canvas.bounds()).contains(x, y))
    }

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

    pub fn focusable(&mut self, id: Id) -> bool {
        self.stop(id, false)
    }

    pub fn focusable_text(&mut self, id: Id) -> bool {
        self.stop(id, true)
    }

    fn stop(&mut self, id: Id, takes_text: bool) -> bool {
        if self.state.modal && !self.in_popup {
            return false;
        }
        self.state.stops.push(Stop { id, takes_text });
        self.state.key_focus == Some(id)
    }

    pub fn activated(&mut self, focused: bool) -> bool {
        focused && (self.take_key(Key::Enter) || self.take_key(Key::Space))
    }

    pub fn take_key(&mut self, key: Key) -> bool {
        let free = self.input.key(key) && !self.state.used_keys.contains(&key);
        if free {
            self.state.used_keys.push(key);
        }
        free
    }

    pub fn step_focus(&mut self, backwards: bool) {
        self.state.step_focus(backwards);
    }

    pub fn focus_ring(&mut self, area: Rect) {
        self.canvas.frame(area.inset(1), self.theme.roles.ink);
    }

    pub fn tooltip(&mut self, text: &str) {
        if let Some(pointer) = self.input.pointer {
            self.tooltip = Some((text.to_owned(), pointer));
        }
    }

    pub fn finish(&mut self) {
        if let Some((text, (x, y))) = self.tooltip.take() {
            let was_on_top = self.canvas.set_on_top(true);
            crate::widgets::draw_tooltip(self, &text, x, y);
            self.canvas.set_on_top(was_on_top);
        }
    }
}
