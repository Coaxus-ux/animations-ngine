/// Index of a color inside a ramp; 0 is the darkest.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Tone(u8);

impl Tone {
    /// Creates a tone from its index. Ramps validate the index when resolving it.
    #[must_use]
    pub const fn new(index: u8) -> Self {
        Self(index)
    }

    /// The index inside the ramp.
    #[must_use]
    pub const fn index(self) -> u8 {
        self.0
    }
}

/// Named tones of the standard 5-tone ramp (`docs/PALETTES.md` §1.1).
///
/// On ramps of other lengths the names map proportionally and `Base` is always the
/// middle tone.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ToneName {
    /// Core shadows (darkest).
    Deep,
    /// Form shadow.
    Shade,
    /// The material's main color.
    Base,
    /// Lit side.
    Lit,
    /// Highlights (lightest).
    Hi,
}

impl ToneName {
    /// Parses `deep`, `shade`, `base`, `lit` or `hi`.
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

    /// Resolves the name on a ramp of `ramp_len` tones; `None` for an empty ramp.
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
