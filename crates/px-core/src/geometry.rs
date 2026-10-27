/// Coordinates are clamped to `±COORD_LIMIT` pixels so every drawing loop stays bounded.
pub const COORD_LIMIT: i32 = 1 << 15;

/// Values this close to a whole pixel snap to it instead of flooring.
///
/// Periodic expressions land on integers only approximately (`sin(2π)` is about
/// `-2.4e-16`), and a plain floor would turn that noise into a one-pixel jump that
/// breaks loops.
const SNAP_EPSILON: f64 = 1e-6;

/// Snaps a coordinate to the pixel grid (floor, with a 1e-6 tolerance) and
/// clamps it into `±COORD_LIMIT`.
///
/// NaN maps to 0; callers are expected to reject non-finite values before drawing.
#[must_use]
pub fn snap_coord(value: f64) -> i32 {
    clamp_to_i32(value, -f64::from(COORD_LIMIT))
}

/// Snaps a size (width, height, radius) like [`snap_coord`], clamped into `0..=COORD_LIMIT`.
#[must_use]
pub fn snap_extent(value: f64) -> i32 {
    clamp_to_i32(value, 0.0)
}

fn clamp_to_i32(value: f64, min: f64) -> i32 {
    if value.is_nan() {
        return 0;
    }
    let nearest = libm::round(value);
    let snapped = if libm::fabs(value - nearest) < SNAP_EPSILON {
        nearest
    } else {
        libm::floor(value)
    };
    let clamped = snapped.clamp(min, f64::from(COORD_LIMIT));
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the value is integral and clamped into ±COORD_LIMIT, which fits i32"
    )]
    let pixel = clamped as i32;
    pixel
}
