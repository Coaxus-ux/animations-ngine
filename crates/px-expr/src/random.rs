use core::f64::consts::TAU;

use px_core::{hash_seed as hash, mix};

const MAX_OCTAVES: u32 = 8;

#[expect(
    clippy::cast_precision_loss,
    reason = "53 high bits fit an f64 mantissa exactly"
)]
pub(crate) fn unit(bits: u64) -> f64 {
    (bits >> 11) as f64 / (1u64 << 53) as f64
}

pub(crate) fn rand(seed: u64, key: f64) -> f64 {
    unit(hash(seed, &[key.to_bits()]))
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "lattice coordinates are floored and clamped to i64 range"
)]
fn cell(value: f64) -> i64 {
    libm::floor(value).clamp(-9.0e15, 9.0e15) as i64
}

fn lattice(seed: u64, cell: [i64; 3]) -> f64 {
    unit(hash(seed, &cell.map(i64::cast_unsigned)))
}

fn smooth(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

pub(crate) fn noise(seed: u64, p: [f64; 3]) -> f64 {
    let base = p.map(cell);
    let frac = [0, 1, 2].map(|axis| smooth(p[axis] - libm::floor(p[axis])));
    let corner =
        |dx: i64, dy: i64, dz: i64| lattice(seed, [base[0] + dx, base[1] + dy, base[2] + dz]);
    let face = |dz: i64| {
        let bottom = lerp(corner(0, 0, dz), corner(1, 0, dz), frac[0]);
        let top = lerp(corner(0, 1, dz), corner(1, 1, dz), frac[0]);
        lerp(bottom, top, frac[1])
    };
    lerp(face(0), face(1), frac[2])
}

pub(crate) fn fbm(seed: u64, x: f64, y: f64, octaves: f64) -> f64 {
    let octaves = cell(octaves).clamp(1, i64::from(MAX_OCTAVES));
    let (mut sum, mut weight, mut amplitude, mut frequency) = (0.0, 0.0, 1.0, 1.0);
    for octave in 0..octaves {
        let layer_seed = seed.wrapping_add(octave.cast_unsigned());
        sum += amplitude * noise(layer_seed, [x * frequency, y * frequency, 0.0]);
        weight += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    sum / weight
}

pub(crate) fn cells(seed: u64, x: f64, y: f64) -> f64 {
    let (cx, cy) = (cell(x), cell(y));
    let mut nearest = f64::MAX;
    for dy in -1..=1 {
        for dx in -1..=1 {
            let bits = hash(
                seed,
                &[(cx + dx).cast_unsigned(), (cy + dy).cast_unsigned()],
            );
            let px = f64::from(i32::try_from(cx + dx).unwrap_or(0)) + unit(bits);
            let py = f64::from(i32::try_from(cy + dy).unwrap_or(0)) + unit(mix(bits));
            nearest = nearest.min(libm::hypot(px - x, py - y));
        }
    }
    (nearest / core::f64::consts::SQRT_2).min(1.0)
}

// A looping scene samples time noise on a circle whose circumference matches the loop, so the
// last frame flows into the first while keeping `freq` wiggles per second.
pub(crate) fn wiggle(
    seed: u64,
    time: f64,
    amplitude: f64,
    frequency: f64,
    loop_period: Option<f64>,
) -> f64 {
    let value = match loop_period.filter(|period| *period > 0.0) {
        Some(period) => {
            let radius = frequency * period / TAU;
            let angle = TAU * time / period;
            noise(
                seed,
                [libm::cos(angle) * radius, libm::sin(angle) * radius, 0.0],
            )
        }
        None => noise(seed, [time * frequency, 0.0, 0.0]),
    };
    amplitude * (2.0 * value - 1.0)
}
