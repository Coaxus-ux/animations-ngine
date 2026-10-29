use core::f64::consts::TAU;

use px_core::{absorb, floor, hash_seed as hash, mix, seeded};

use crate::program::Wind;

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
    floor(value).clamp(-9.0e15, 9.0e15) as i64
}

fn smooth(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

pub(crate) fn noise(seed: u64, p: [f64; 3]) -> f64 {
    let base = p.map(cell);
    let frac = [0, 1, 2].map(|axis| smooth(p[axis] - floor(p[axis])));
    // Lattice corners hash (seed, x, y, z) in that order, so each x column is hashed once.
    let root = seeded(seed);
    let columns = [0, 1].map(|dx| absorb(root, (base[0] + dx).cast_unsigned()));
    let face = |dz: i64| {
        let z = (base[2] + dz).cast_unsigned();
        let corner =
            |column: u64, dy: i64| unit(absorb(absorb(column, (base[1] + dy).cast_unsigned()), z));
        let bottom = lerp(corner(columns[0], 0), corner(columns[1], 0), frac[0]);
        let top = lerp(corner(columns[0], 1), corner(columns[1], 1), frac[0]);
        lerp(bottom, top, frac[1])
    };
    // `lerp(a, b, 0.0)` is exactly `a`: integral z (every 2D caller) needs only one face.
    if frac[2] == 0.0 {
        return face(0);
    }
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
    let root = seeded(seed);
    let mut nearest = f64::MAX;
    for dy in -1..=1 {
        for dx in -1..=1 {
            let bits = absorb(
                absorb(root, (cx + dx).cast_unsigned()),
                (cy + dy).cast_unsigned(),
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

const WIND_SEED: u64 = 0x5749_4e44;
const WIND_CELL: f64 = 32.0;

// Gusts travel along x; in looping scenes time is sampled on a circle so the field loops too.
pub(crate) fn wind(field: Wind, time: f64, x: f64, loop_period: Option<f64>) -> f64 {
    let along = x / WIND_CELL;
    let gust = match loop_period.filter(|period| *period > 0.0) {
        Some(period) => {
            let radius = field.speed * period / (TAU * WIND_CELL);
            let angle = TAU * time / period;
            noise(
                WIND_SEED,
                [
                    along + libm::cos(angle) * radius,
                    libm::sin(angle) * radius,
                    0.0,
                ],
            )
        }
        None => noise(
            WIND_SEED,
            [along - time * field.speed / WIND_CELL, 0.5, 0.0],
        ),
    };
    field.strength * (1.0 + field.gust * (2.0 * gust - 1.0))
}

pub(crate) fn value_noise(seed: u64, x: f64, y: f64) -> f64 {
    noise(seed, [x, y, 0.0])
}
