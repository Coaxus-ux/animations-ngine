#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Tone(u8);

impl Tone {
    #[must_use]
    pub const fn new(index: u8) -> Self {
        Self(index)
    }

    #[must_use]
    pub const fn index(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ToneName {
    Deep,
    Shade,
    Base,
    Lit,
    Hi,
}

impl ToneName {
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "deep" => Some(Self::Deep),
            "shade" => Some(Self::Shade),
            "base" => Some(Self::Base),
            "lit" => Some(Self::Lit),
            "hi" => Some(Self::Hi),
            _ => None,
        }
    }

    #[must_use]
    pub fn resolve(self, ramp_len: u8) -> Option<Tone> {
        let last = ramp_len.checked_sub(1)?;
        let base = last / 2;
        let index = match self {
            Self::Deep => 0,
            Self::Shade => base / 2,
            Self::Base => base,
            Self::Lit => base + (last - base).div_ceil(2),
            Self::Hi => last,
        };
        debug_assert!(index <= last, "named tone resolved outside its ramp");
        Some(Tone::new(index))
    }
}
