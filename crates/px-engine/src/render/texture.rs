use super::mask::Mask;
use super::transform::Transform;
use crate::model::Texture;

/// Noise is sampled in local coordinates, so it moves and scales with the shape instead of
/// swimming when the camera pans or zooms.
pub(crate) fn sample_texture(
    texture: Texture,
    seed: u64,
    transform: &Transform,
    mask: &mut Mask<'_>,
) {
    let scale = texture.scale.max(1.0);
    for (x, y) in mask.pixels() {
        if let Some(index) = mask.index(x, y) {
            let [lx, ly] = transform.local([f64::from(x), f64::from(y)]);
            let n = px_expr::value_noise(seed, lx / scale, ly / scale);
            mask.noise[index] = if n > 1.0 - texture.amount / 2.0 {
                1
            } else if n < texture.amount / 2.0 {
                -1
            } else {
                0
            };
        }
    }
}

// A 3×3 majority vote groups noise into clumps instead of static (RENDERING §7.4).
pub(crate) fn clustered_noise(mask: &Mask<'_>, x: i32, y: i32) -> i8 {
    let raw = |dx: i32, dy: i32| {
        mask.index(x + dx, y + dy)
            .filter(|_| mask.covered(x + dx, y + dy))
            .map_or(0, |i| mask.noise[i])
    };
    let mut counts = [0u8; 3];
    for dy in -1..=1 {
        for dx in -1..=1 {
            counts[usize::try_from(raw(dx, dy) + 1).unwrap_or(1)] += 1;
        }
    }
    match counts.iter().position(|count| *count >= 5) {
        Some(winner) => i8::try_from(winner).unwrap_or(1) - 1,
        None => raw(0, 0),
    }
}
