/// Width of the dithered seam between two solid bands, as a fraction of one tone step.
const SEAM: f64 = 0.3;

const BAYER: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// Ordered-dither threshold in (0, 1) for a pixel; anchored to canvas coordinates.
#[must_use]
pub fn bayer_threshold(x: i32, y: i32) -> f64 {
    let level = BAYER[(y & 3).unsigned_abs() as usize][(x & 3).unsigned_abs() as usize];
    (f64::from(level) + 0.5) / 16.0
}

#[must_use]
pub fn dither_keep(amount: f64, x: i32, y: i32) -> bool {
    amount >= 1.0 || amount > bayer_threshold(x, y)
}

/// Whole tone steps for `value`: solid bands, ordered dither only in a narrow seam
/// around each half step, as pixel artists do.
#[must_use]
pub fn dither_steps(value: f64, x: i32, y: i32) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    let whole = crate::floor(value);
    let fraction = ((value - whole - (1.0 - SEAM) / 2.0) / SEAM).clamp(0.0, 1.0);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "tone steps are clamped to a few hundred"
    )]
    let whole = whole.clamp(-512.0, 512.0) as i32;
    whole + i32::from(fraction > bayer_threshold(x, y))
}
