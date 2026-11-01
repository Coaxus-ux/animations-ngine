const INTEGRAL_FROM: f64 = 4_503_599_627_370_496.0;

#[inline]
#[must_use]
pub fn floor(value: f64) -> f64 {
    if value.is_nan() || value.abs() >= INTEGRAL_FROM {
        return value;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "|value| < 2^52, so the truncated integer round-trips through i64 exactly"
    )]
    let truncated = (value as i64) as f64;
    let floored = if truncated > value {
        truncated - 1.0
    } else {
        truncated
    };
    floored.copysign(value)
}
