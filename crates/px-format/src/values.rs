//! Converts KDL entry values into typed values, reporting problems as diagnostics.

use std::fmt::Display;

use kdl::{KdlEntry, KdlValue};
use px_core::{FrameCount, FrameRate, MAX_FRAMES};
use px_engine::Param;

use crate::diagnostic::Diagnostics;

fn offset(entry: &KdlEntry) -> usize {
    entry.span().offset()
}

/// Converts an integer literal to `f64`.
#[expect(
    clippy::cast_precision_loss,
    reason = "KDL integers in scene files are small; f64 is the engine's number type"
)]
fn integer_to_f64(value: i128) -> f64 {
    value as f64
}

/// A number literal.
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

/// An integer within `min..=max`.
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

/// A boolean (`#true` / `#false`).
pub(crate) fn boolean(entry: &KdlEntry, diags: &mut Diagnostics) -> Option<bool> {
    let value = entry.value().as_bool();
    if value.is_none() {
        diags.error(offset(entry), "expected #true or #false");
    }
    value
}

/// A string.
pub(crate) fn string<'a>(entry: &'a KdlEntry, diags: &mut Diagnostics) -> Option<&'a str> {
    let value = entry.value().as_string();
    if value.is_none() {
        diags.error(offset(entry), "expected a string");
    }
    value
}

/// A numeric parameter: a number, or a string starting with `=` holding an expression.
pub(crate) fn param(entry: &KdlEntry, diags: &mut Diagnostics) -> Option<Param> {
    let Some(text) = entry.value().as_string() else {
        return number(entry, diags).map(Param::Fixed);
    };
    let Some(source) = text.strip_prefix('=') else {
        diags.error(
            offset(entry),
            format!("expected a number or \"=expression\", found \"{text}\""),
        );
        return None;
    };
    match px_expr::compile(source) {
        Ok(program) => Some(Param::Expr(program)),
        Err(error) => {
            let snippet = source.get(error.start..error.end).unwrap_or("");
            diags.error(
                offset(entry),
                format!(
                    "invalid expression `{source}`: {} (`{snippet}` at character {})",
                    error.kind,
                    error.start + 1
                ),
            );
            None
        }
    }
}

/// A duration with an explicit unit (`"2s"`, `"500ms"`, `"24f"`), as whole frames.
/// A duration that is not a whole number of frames is rounded, with a warning.
pub(crate) fn duration(
    entry: &KdlEntry,
    fps: FrameRate,
    diags: &mut Diagnostics,
) -> Option<FrameCount> {
    let text = string(entry, diags)?;
    let Some(frames) = frames_of(text, fps) else {
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

fn frames_of(text: &str, fps: FrameRate) -> Option<f64> {
    let fps = f64::from(fps.get());
    let (number, scale) = if let Some(number) = text.strip_suffix("ms") {
        (number, fps / 1000.0)
    } else if let Some(number) = text.strip_suffix('s') {
        (number, fps)
    } else {
        (text.strip_suffix('f')?, 1.0)
    };
    let value: f64 = number.parse().ok()?;
    (value.is_finite() && value >= 0.0).then_some(value * scale)
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "range-checked integral value"
)]
fn whole_frames(value: f64) -> Option<u32> {
    (value >= 0.0 && value <= f64::from(MAX_FRAMES)).then_some(value as u32)
}
