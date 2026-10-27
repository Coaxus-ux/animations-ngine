use core::fmt;

use crate::color::Rgb;
use crate::tone::{Tone, ToneName};

pub const MAX_RAMPS: usize = 256;
pub const MAX_TONES: usize = 8;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RampId(u16);

impl RampId {
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Paint {
    pub ramp: RampId,
    pub tone: Tone,
}

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

    #[must_use]
    pub fn colors(&self) -> &[Rgb] {
        &self.colors[..usize::from(self.len)]
    }

    #[must_use]
    pub const fn len(&self) -> u8 {
        self.len
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[must_use]
    pub fn named(&self, name: ToneName) -> Option<Tone> {
        name.resolve(self.len)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaletteError {
    Full { capacity: usize },
    EmptyRamp,
    RampTooLong { len: usize, max: usize },
    UnknownRamp { ramp: RampId },
    ToneOutOfRange { ramp: RampId, tone: Tone, len: u8 },
}

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
    #[must_use]
    pub const fn new() -> Self {
        Self {
            ramps: [Ramp::EMPTY; MAX_RAMPS],
            len: 0,
        }
    }

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

    #[must_use]
    pub fn ramps(&self) -> &[Ramp] {
        &self.ramps[..usize::from(self.len)]
    }

    pub fn ramp(&self, id: RampId) -> Result<&Ramp, PaletteError> {
        self.ramps()
            .get(id.index())
            .ok_or(PaletteError::UnknownRamp { ramp: id })
    }

    pub fn paint(&self, ramp: RampId, tone: Tone) -> Result<Paint, PaletteError> {
        let len = self.ramp(ramp)?.len();
        if tone.index() >= len {
            return Err(PaletteError::ToneOutOfRange { ramp, tone, len });
        }
        Ok(Paint { ramp, tone })
    }

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
