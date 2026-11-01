#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Key {
    Letter(char),
    Digit(u8),
    Function(u8),
    Escape,
    Enter,
    Tab,
    Space,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Plus,
    Minus,
    Comma,
    Period,
    BracketLeft,
    BracketRight,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Buttons {
    pub left: bool,
    pub right: bool,
    pub middle: bool,
}

impl Buttons {
    #[must_use]
    pub const fn any(self) -> bool {
        self.left || self.right || self.middle
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each modifier key is independent"
)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub command: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Input {
    pub pointer: Option<(i32, i32)>,
    pub pointer_anywhere: (i32, i32),
    pub down: Buttons,
    pub pressed: Buttons,
    pub released: Buttons,
    pub wheel: (f32, f32),
    pub keys: Vec<Key>,
    pub held: Vec<Key>,
    pub modifiers: Modifiers,
    pub text: String,
    pub time_ms: u64,
}

impl Input {
    #[must_use]
    pub fn key(&self, key: Key) -> bool {
        self.keys.contains(&key)
    }

    #[must_use]
    pub fn holding(&self, key: Key) -> bool {
        self.held.contains(&key)
    }

    #[must_use]
    pub fn shortcut(&self, key: Key) -> bool {
        self.modifiers.command && self.key(key)
    }

    #[must_use]
    pub fn is_idle(&self) -> bool {
        !self.down.any()
            && !self.pressed.any()
            && !self.released.any()
            && self.wheel == (0.0, 0.0)
            && self.keys.is_empty()
            && self.text.is_empty()
    }
}
