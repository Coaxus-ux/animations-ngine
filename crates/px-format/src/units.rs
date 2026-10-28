use px_core::{FrameRate, Size};

use crate::timeline::Markers;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Unit {
    None,
    Px,
    Percent,
    Frames,
    Seconds,
    Millis,
    Degrees,
    HeadUnits,
    Tiles,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Quantity {
    Length(Axis),
    Number,
    Angle,
    Time,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Literal {
    pub(crate) value: f64,
    pub(crate) unit: Unit,
    pub(crate) integer: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Measure<'m> {
    pub(crate) canvas: Size,
    pub(crate) fps: FrameRate,
    pub(crate) markers: &'m Markers,
}

impl Measure<'_> {
    /// A number with its unit, or the time of a timeline marker.
    pub(crate) fn literal(&self, text: &str) -> Option<Literal> {
        literal(text).or_else(|| {
            self.markers.get(text.trim()).map(|seconds| Literal {
                value: seconds,
                unit: Unit::Seconds,
                integer: false,
            })
        })
    }
}

const SUFFIXES: [(&str, Unit); 8] = [
    ("px", Unit::Px),
    ("%", Unit::Percent),
    ("ms", Unit::Millis),
    ("s", Unit::Seconds),
    ("f", Unit::Frames),
    ("deg", Unit::Degrees),
    ("hu", Unit::HeadUnits),
    ("tile", Unit::Tiles),
];

pub(crate) fn literal(text: &str) -> Option<Literal> {
    let text = text.trim();
    let (number, unit) = SUFFIXES
        .iter()
        .find_map(|(suffix, unit)| text.strip_suffix(suffix).map(|number| (number, *unit)))
        .unwrap_or((text, Unit::None));
    let value: f64 = number
        .parse()
        .ok()
        .filter(|value: &f64| value.is_finite())?;
    let integer = !number.contains(['.', 'e', 'E']);
    Some(Literal {
        value,
        unit,
        integer,
    })
}

impl Unit {
    const fn name(self) -> &'static str {
        match self {
            Self::None => "no unit",
            Self::Px => "px",
            Self::Percent => "%",
            Self::Frames => "f",
            Self::Seconds => "s",
            Self::Millis => "ms",
            Self::Degrees => "deg",
            Self::HeadUnits => "hu",
            Self::Tiles => "tile",
        }
    }
}

impl Literal {
    pub(crate) fn convert(self, quantity: Quantity, measure: Measure<'_>) -> Result<f64, String> {
        match (quantity, self.unit) {
            (Quantity::Length(axis), unit) => self.length(axis, unit, measure),
            (Quantity::Number, Unit::None) | (Quantity::Angle, Unit::None | Unit::Degrees) => {
                Ok(self.value)
            }
            (Quantity::Number, Unit::Percent) => Ok(self.value / 100.0),
            (Quantity::Time, _) => self
                .seconds(measure.fps)
                .ok_or_else(|| self.wrong("a time (use s, ms or f)")),
            (Quantity::Angle, _) => Err(self.wrong("an angle (use deg)")),
            (Quantity::Number, _) => Err(self.wrong("a plain number")),
        }
    }

    fn length(self, axis: Axis, unit: Unit, measure: Measure<'_>) -> Result<f64, String> {
        let side = match axis {
            Axis::Horizontal => measure.canvas.width(),
            Axis::Vertical => measure.canvas.height(),
        };
        match unit {
            Unit::None | Unit::Px => Ok(self.value),
            Unit::Percent => Ok(self.value / 100.0 * f64::from(side)),
            Unit::HeadUnits => Err("`hu` is only available inside body plans".to_owned()),
            Unit::Tiles => Err("`tile` is only available with a tileset".to_owned()),
            other => Err(format!("`{}` is not a length (use px or %)", other.name())),
        }
    }

    fn wrong(self, expected: &str) -> String {
        format!("`{}` is not {expected}", self.unit.name())
    }

    pub(crate) fn seconds(self, fps: FrameRate) -> Option<f64> {
        match self.unit {
            Unit::None | Unit::Seconds => Some(self.value),
            _ => self.frames(fps).map(|frames| frames / f64::from(fps.get())),
        }
    }

    pub(crate) fn frames(self, fps: FrameRate) -> Option<f64> {
        let fps = f64::from(fps.get());
        match self.unit {
            Unit::Frames => Some(self.value),
            Unit::Seconds => Some(self.value * fps),
            Unit::Millis => Some(self.value * fps / 1000.0),
            _ => None,
        }
    }
}
