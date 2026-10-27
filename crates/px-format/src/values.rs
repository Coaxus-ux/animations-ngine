use std::fmt::Display;

use kdl::{KdlEntry, KdlValue};
use px_core::{FrameCount, FrameRate, MAX_FRAMES};

use crate::diagnostic::Diagnostics;
use crate::units;

fn offset(entry: &KdlEntry) -> usize {
    entry.span().offset()
}

#[expect(
    clippy::cast_precision_loss,
    reason = "KDL integers in scene files are small; f64 is the engine's number type"
)]
pub(crate) fn integer_to_f64(value: i128) -> f64 {
    value as f64
}

pub(crate) fn number(entry: &KdlEntry, diags: &mut Diagnostics) -> Option<f64> {
    match entry.value() {
        KdlValue::Integer(value) => Some(integer_to_f64(*value)),
        KdlValue::Float(value) if value.is_finite() => Some(*value),
        _ => {
            diags.error(offset(entry), "expected a number");
            None
        }
    }
}

pub(crate) fn bounded<T>(entry: &KdlEntry, min: T, max: T, diags: &mut Diagnostics) -> Option<T>
where
    T: TryFrom<i128> + PartialOrd + Display + Copy,
{
    let value = entry
        .value()
        .as_integer()
        .and_then(|value| T::try_from(value).ok())
        .filter(|value| *value >= min && *value <= max);
    if value.is_none() {
        diags.error(
            offset(entry),
            format!("expected a whole number in {min}..={max}"),
        );
    }
    value
}

pub(crate) fn boolean(entry: &KdlEntry, diags: &mut Diagnostics) -> Option<bool> {
    let value = entry.value().as_bool();
    if value.is_none() {
        diags.error(offset(entry), "expected #true or #false");
    }
    value
}

pub(crate) fn string<'a>(entry: &'a KdlEntry, diags: &mut Diagnostics) -> Option<&'a str> {
    let value = entry.value().as_string();
    if value.is_none() {
        diags.error(offset(entry), "expected a string");
    }
    value
}

pub(crate) fn duration(
    entry: &KdlEntry,
    fps: FrameRate,
    diags: &mut Diagnostics,
) -> Option<FrameCount> {
    let text = string(entry, diags)?;
    let frames = units::literal(text)
        .and_then(|literal| literal.frames(fps))
        .filter(|frames| *frames >= 0.0);
    let Some(frames) = frames else {
        diags.error(
            offset(entry),
            format!("invalid duration \"{text}\": use a unit like \"2s\", \"500ms\" or \"24f\""),
        );
        return None;
    };
    let rounded = frames.round();
    if (rounded - frames).abs() > 1e-9 {
        diags.warning(
            offset(entry),
            format!("duration \"{text}\" is not a whole number of frames; rounded to {rounded}f"),
        );
    }
    let count = whole_frames(rounded).and_then(|frames| FrameCount::new(frames).ok());
    if count.is_none() {
        diags.error(
            offset(entry),
            format!("duration must be 1..={MAX_FRAMES} frames"),
        );
    }
    count
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "range-checked integral value"
)]
fn whole_frames(value: f64) -> Option<u32> {
    (value >= 0.0 && value <= f64::from(MAX_FRAMES)).then_some(value as u32)
}
