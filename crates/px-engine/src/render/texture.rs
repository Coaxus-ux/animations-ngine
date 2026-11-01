use super::mask::Mask;
use super::transform::Transform;
use crate::model::Texture;

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

pub(crate) fn clustered_noise(mask: &Mask<'_>, x: i32, y: i32) -> i8 {
    let raw = |index: usize| {
        if mask.coverage[index] > 0 {
            mask.noise[index]
        } else {
            0
        }
    };
    let vote = |noise: i8| usize::try_from(noise + 1).unwrap_or(1);
    let (left, right) = ((x - 1).max(0), (x + 1).min(mask.width - 1));
    let mut counts = [0u8, 9, 0];
    for row in (y - 1).max(0)..=(y + 1).min(mask.height - 1) {
        if let (Some(start), Some(end)) = (mask.index(left, row), mask.index(right, row)) {
            for index in start..=end {
                counts[1] -= 1;
                counts[vote(raw(index))] += 1;
            }
        }
    }
    match counts.iter().position(|count| *count >= 5) {
        Some(winner) => i8::try_from(winner).unwrap_or(1) - 1,
        None => mask.index(x, y).map_or(0, raw),
    }
}
