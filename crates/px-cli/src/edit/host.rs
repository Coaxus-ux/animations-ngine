use std::cell::RefCell;
use std::rc::Rc;

use minifb::{
    InputCallback, Key as WinKey, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions,
};
use px_ui::{Buttons, Input, Key, Modifiers};

pub(super) struct Host {
    pub(super) window: Window,
    typed: Rc<RefCell<String>>,
    down: Buttons,
}

struct Typed(Rc<RefCell<String>>);

impl InputCallback for Typed {
    fn add_char(&mut self, character: u32) {
        if let Some(character) = char::from_u32(character) {
            self.0.borrow_mut().push(character);
        }
    }
}

impl Host {
    pub(super) fn open(title: &str, width: usize, height: usize) -> Result<Self, String> {
        let options = WindowOptions {
            resize: true,
            ..WindowOptions::default()
        };
        let mut window =
            Window::new(title, width, height, options).map_err(|error| error.to_string())?;
        window.set_target_fps(0);
        let typed = Rc::new(RefCell::new(String::new()));
        window.set_input_callback(Box::new(Typed(Rc::clone(&typed))));
        Ok(Self {
            window,
            typed,
            down: Buttons::default(),
        })
    }

    pub(super) fn input(&mut self, time_ms: u64) -> Input {
        let window = &self.window;
        let held: Vec<WinKey> = window.get_keys();
        let holding = |keys: &[WinKey]| keys.iter().any(|key| held.contains(key));
        let modifiers = Modifiers {
            shift: holding(&[WinKey::LeftShift, WinKey::RightShift]),
            ctrl: holding(&[WinKey::LeftCtrl, WinKey::RightCtrl]),
            alt: holding(&[WinKey::LeftAlt, WinKey::RightAlt]),
            command: holding(&[
                WinKey::LeftCtrl,
                WinKey::RightCtrl,
                WinKey::LeftSuper,
                WinKey::RightSuper,
            ]),
        };
        let down = Buttons {
            left: window.get_mouse_down(MouseButton::Left),
            right: window.get_mouse_down(MouseButton::Right),
            middle: window.get_mouse_down(MouseButton::Middle),
        };
        let edge = |now: bool, before: bool| (now && !before, !now && before);
        let (left_pressed, left_released) = edge(down.left, self.down.left);
        let (right_pressed, right_released) = edge(down.right, self.down.right);
        let (middle_pressed, middle_released) = edge(down.middle, self.down.middle);
        self.down = down;
        let position = |mode| {
            window
                .get_mouse_pos(mode)
                .map(|(x, y): (f32, f32)| (whole(x), whole(y)))
        };
        let mut text = core::mem::take(&mut *self.typed.borrow_mut());
        if modifiers.command {
            text.clear();
        }
        Input {
            pointer: position(MouseMode::Discard),
            pointer_anywhere: position(MouseMode::Pass).unwrap_or_default(),
            down,
            pressed: Buttons {
                left: left_pressed,
                right: right_pressed,
                middle: middle_pressed,
            },
            released: Buttons {
                left: left_released,
                right: right_released,
                middle: middle_released,
            },
            wheel: window.get_scroll_wheel().unwrap_or_default(),
            keys: window
                .get_keys_pressed(KeyRepeat::Yes)
                .into_iter()
                .filter_map(key)
                .collect(),
            held: held.into_iter().filter_map(key).collect(),
            modifiers,
            text,
            time_ms,
        }
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "window coordinates are small"
)]
fn whole(value: f32) -> i32 {
    value.floor() as i32
}

fn key(key: WinKey) -> Option<Key> {
    const LETTERS: [(WinKey, char); 26] = [
        (WinKey::A, 'a'),
        (WinKey::B, 'b'),
        (WinKey::C, 'c'),
        (WinKey::D, 'd'),
        (WinKey::E, 'e'),
        (WinKey::F, 'f'),
        (WinKey::G, 'g'),
        (WinKey::H, 'h'),
        (WinKey::I, 'i'),
        (WinKey::J, 'j'),
        (WinKey::K, 'k'),
        (WinKey::L, 'l'),
        (WinKey::M, 'm'),
        (WinKey::N, 'n'),
        (WinKey::O, 'o'),
        (WinKey::P, 'p'),
        (WinKey::Q, 'q'),
        (WinKey::R, 'r'),
        (WinKey::S, 's'),
        (WinKey::T, 't'),
        (WinKey::U, 'u'),
        (WinKey::V, 'v'),
        (WinKey::W, 'w'),
        (WinKey::X, 'x'),
        (WinKey::Y, 'y'),
        (WinKey::Z, 'z'),
    ];
    const DIGITS: [WinKey; 10] = [
        WinKey::Key0,
        WinKey::Key1,
        WinKey::Key2,
        WinKey::Key3,
        WinKey::Key4,
        WinKey::Key5,
        WinKey::Key6,
        WinKey::Key7,
        WinKey::Key8,
        WinKey::Key9,
    ];
    const FUNCTIONS: [WinKey; 12] = [
        WinKey::F1,
        WinKey::F2,
        WinKey::F3,
        WinKey::F4,
        WinKey::F5,
        WinKey::F6,
        WinKey::F7,
        WinKey::F8,
        WinKey::F9,
        WinKey::F10,
        WinKey::F11,
        WinKey::F12,
    ];
    if let Some((_, letter)) = LETTERS.iter().find(|(win, _)| *win == key) {
        return Some(Key::Letter(*letter));
    }
    if let Some(digit) = DIGITS.iter().position(|win| *win == key) {
        return u8::try_from(digit).ok().map(Key::Digit);
    }
    if let Some(function) = FUNCTIONS.iter().position(|win| *win == key) {
        return u8::try_from(function + 1).ok().map(Key::Function);
    }
    Some(match key {
        WinKey::Escape => Key::Escape,
        WinKey::Enter | WinKey::NumPadEnter => Key::Enter,
        WinKey::Tab => Key::Tab,
        WinKey::Space => Key::Space,
        WinKey::Backspace => Key::Backspace,
        WinKey::Delete => Key::Delete,
        WinKey::Left => Key::Left,
        WinKey::Right => Key::Right,
        WinKey::Up => Key::Up,
        WinKey::Down => Key::Down,
        WinKey::Home => Key::Home,
        WinKey::End => Key::End,
        WinKey::PageUp => Key::PageUp,
        WinKey::PageDown => Key::PageDown,
        WinKey::Equal | WinKey::NumPadPlus => Key::Plus,
        WinKey::Minus | WinKey::NumPadMinus => Key::Minus,
        WinKey::Comma => Key::Comma,
        WinKey::Period => Key::Period,
        WinKey::LeftBracket => Key::BracketLeft,
        WinKey::RightBracket => Key::BracketRight,
        _ => return None,
    })
}
