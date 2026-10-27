pub const COORD_LIMIT: i32 = 1 << 15;

const SNAP_EPSILON: f64 = 1e-6;

#[must_use]
pub fn snap_coord(value: f64) -> i32 {
    clamp_to_i32(value, -f64::from(COORD_LIMIT))
}

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
