use core::fmt;

use crate::color::Rgb;
use crate::tone::{Tone, ToneName};

/// Maximum number of ramps in one palette.
pub const MAX_RAMPS: usize = 256;
/// Maximum number of tones in one ramp.
pub const MAX_TONES: usize = 8;

/// Identifies a ramp inside its palette.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RampId(u16);

impl RampId {
    /// The ramp's position in its palette.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// What a pixel stores: a ramp (material) and a tone on that ramp.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Paint {
    /// The material's ramp.
    pub ramp: RampId,
    /// The tone on the ramp.
    pub tone: Tone,
}

/// An ordered list of colors from dark to light for one material.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ramp {
    colors: [Rgb; MAX_TONES],
    len: u8,
}

impl Ramp {
    const EMPTY: Self = Self {
        colors: [Rgb::new(0, 0, 0); MAX_TONES],
        len: 0,
    };

    /// The ramp's colors, darkest first.
    #[must_use]
    pub fn colors(&self) -> &[Rgb] {
        &self.colors[..usize::from(self.len)]
    }

    /// Number of tones; at least 1 for every ramp stored in a palette.
    #[must_use]
    pub const fn len(&self) -> u8 {
        self.len
    }

    /// Whether the ramp has no tones (never true for ramps stored in a palette).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Resolves a named tone on this ramp.
    #[must_use]
    pub fn named(&self, name: ToneName) -> Option<Tone> {
        name.resolve(self.len)
    }
}

/// Why a palette operation failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaletteError {
    /// The palette already holds [`MAX_RAMPS`] ramps.
    Full {
        /// The fixed capacity.
        capacity: usize,
    },
    /// A ramp must have at least one color.
    EmptyRamp,
    /// A ramp has more than [`MAX_TONES`] colors.
    RampTooLong {
        /// Colors given.
        len: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// The ramp id does not exist in this palette.
    UnknownRamp {
        /// The missing ramp.
        ramp: RampId,
    },
    /// The tone index is past the end of its ramp.
    ToneOutOfRange {
        /// The ramp.
        ramp: RampId,
        /// The requested tone.
        tone: Tone,
        /// The ramp's length.
        len: u8,
    },
}

/// A fixed-capacity set of ramps. Pixels reference it through [`Paint`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Palette {
    ramps: [Ramp; MAX_RAMPS],
    len: u16,
}

impl Default for Palette {
    fn default() -> Self {
        Self::new()
    }
}

impl Palette {
    /// An empty palette.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            ramps: [Ramp::EMPTY; MAX_RAMPS],
            len: 0,
        }
    }

    /// Adds a ramp (colors darkest first) and returns its id.
    ///
    /// # Errors
    ///
    /// [`PaletteError::Full`], [`PaletteError::EmptyRamp`] or [`PaletteError::RampTooLong`].
    pub fn push_ramp(&mut self, colors: &[Rgb]) -> Result<RampId, PaletteError> {
        let slot = usize::from(self.len);
        if slot >= MAX_RAMPS {
            return Err(PaletteError::Full {
                capacity: MAX_RAMPS,
            });
        }
        let len = u8::try_from(colors.len())
            .ok()
            .filter(|len| usize::from(*len) <= MAX_TONES)
            .ok_or(PaletteError::RampTooLong {
                len: colors.len(),
                max: MAX_TONES,
            })?;
        if len == 0 {
            return Err(PaletteError::EmptyRamp);
        }
        let ramp = &mut self.ramps[slot];
        ramp.colors[..colors.len()].copy_from_slice(colors);
        ramp.len = len;
        let id = RampId(self.len);
        self.len += 1;
        Ok(id)
    }

    /// The ramps in insertion order.
    #[must_use]
    pub fn ramps(&self) -> &[Ramp] {
        &self.ramps[..usize::from(self.len)]
    }

    /// Looks up a ramp.
    ///
    /// # Errors
    ///
    /// [`PaletteError::UnknownRamp`] when the id is not in this palette.
    pub fn ramp(&self, id: RampId) -> Result<&Ramp, PaletteError> {
        self.ramps()
            .get(id.index())
            .ok_or(PaletteError::UnknownRamp { ramp: id })
    }

    /// Builds a paint after checking that the tone exists on the ramp.
    ///
    /// # Errors
    ///
    /// [`PaletteError::UnknownRamp`] or [`PaletteError::ToneOutOfRange`].
    pub fn paint(&self, ramp: RampId, tone: Tone) -> Result<Paint, PaletteError> {
        let len = self.ramp(ramp)?.len();
        if tone.index() >= len {
            return Err(PaletteError::ToneOutOfRange { ramp, tone, len });
        }
        Ok(Paint { ramp, tone })
    }

    /// The RGB color of a paint.
    ///
    /// # Errors
    ///
    /// [`PaletteError::UnknownRamp`] or [`PaletteError::ToneOutOfRange`].
    pub fn resolve(&self, paint: Paint) -> Result<Rgb, PaletteError> {
        let ramp = self.ramp(paint.ramp)?;
        ramp.colors()
            .get(usize::from(paint.tone.index()))
            .copied()
            .ok_or(PaletteError::ToneOutOfRange {
                ramp: paint.ramp,
                tone: paint.tone,
                len: ramp.len(),
            })
    }

    /// Whether the paint resolves in this palette (the palette-compliance check).
    #[must_use]
    pub fn contains(&self, paint: Paint) -> bool {
        self.resolve(paint).is_ok()
    }
}

impl fmt::Display for PaletteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full { capacity } => write!(f, "palette is full ({capacity} ramps)"),
            Self::EmptyRamp => f.write_str("a ramp needs at least one color"),
            Self::RampTooLong { len, max } => write!(f, "ramp has {len} colors, max is {max}"),
            Self::UnknownRamp { ramp } => write!(f, "unknown ramp #{}", ramp.index()),
            Self::ToneOutOfRange { ramp, tone, len } => write!(
                f,
                "tone {} is outside ramp #{} ({len} tones)",
                tone.index(),
                ramp.index()
            ),
        }
    }
}
