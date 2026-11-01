use super::surface::{CASTS_SHADOW, Surface};

const MAX_SHADOW_STEPS: f64 = 160.0;
const BOX_MARGIN: f64 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Casters {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
    deepest: u16,
}

impl Casters {
    pub(crate) fn find(surface: &Surface<'_>) -> Option<Self> {
        let width = usize::try_from(surface.width).ok()?.max(1);
        let rows = surface
            .flags
            .chunks_exact(width)
            .zip(surface.depth.chunks_exact(width));
        let mut found: Option<Self> = None;
        for (y, (flags, depths)) in (0u32..).zip(rows) {
            let casts = |flag: &u8| flag & CASTS_SHADOW != 0;
            let (Some(first), Some(last)) =
                (flags.iter().position(casts), flags.iter().rposition(casts))
            else {
                continue;
            };
            let deepest = flags[first..=last]
                .iter()
                .zip(&depths[first..=last])
                .filter(|(flag, _)| casts(flag))
                .map(|(_, depth)| *depth)
                .max()
                .unwrap_or(0);
            let row = Self {
                left: column(first),
                top: f64::from(y),
                right: column(last) + 1.0,
                bottom: f64::from(y) + 1.0,
                deepest,
            };
            found = Some(found.map_or(row, |seen| seen.union(&row)));
        }
        found.map(|casters| casters.padded())
    }

    pub(crate) fn block(
        &self,
        surface: &Surface<'_>,
        x: i32,
        y: i32,
        (lx, ly): (f64, f64),
    ) -> bool {
        let Some(own) = surface.index(x, y).map(|index| surface.depth[index]) else {
            return false;
        };
        if self.deepest <= own {
            return false;
        }
        let (px, py) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
        let distance = libm::hypot(lx - px, ly - py).min(MAX_SHADOW_STEPS);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "bounded by MAX_SHADOW_STEPS"
        )]
        let steps = distance as u32;
        let Some((first, end)) = self.steps_inside((px, py), (lx - px, ly - py), steps) else {
            return false;
        };
        (first..end).any(|step| {
            let t = f64::from(step) / f64::from(steps);
            #[expect(
                clippy::cast_possible_truncation,
                reason = "on-canvas sample positions fit i32"
            )]
            let (sx, sy) = (
                px_core::floor(px + (lx - px) * t) as i32,
                px_core::floor(py + (ly - py) * t) as i32,
            );
            surface
                .index(sx, sy)
                .is_some_and(|i| surface.flags[i] & CASTS_SHADOW != 0 && surface.depth[i] > own)
        })
    }

    fn steps_inside(&self, start: (f64, f64), delta: (f64, f64), steps: u32) -> Option<(u32, u32)> {
        let end = (start.0 + delta.0, start.1 + delta.1);
        if self.contains(start) && self.contains(end) {
            return (steps > 1).then_some((1, steps));
        }
        let (enter_x, leave_x) = crossing(start.0, delta.0, self.left, self.right)?;
        let (enter_y, leave_y) = crossing(start.1, delta.1, self.top, self.bottom)?;
        let (enter, leave) = (enter_x.max(enter_y).max(0.0), leave_x.min(leave_y).min(1.0));
        if enter > leave {
            return None;
        }
        let total = f64::from(steps);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "enter and leave are within 0..=1, so the step indices are within 0..=steps"
        )]
        let (first, last) = (
            px_core::floor(enter * total) as u32,
            px_core::floor(leave * total) as u32 + 1,
        );
        let (first, end) = (first.max(1), last.saturating_add(1).min(steps));
        (first < end).then_some((first, end))
    }

    fn contains(&self, (x, y): (f64, f64)) -> bool {
        x >= self.left && x <= self.right && y >= self.top && y <= self.bottom
    }

    fn union(&self, other: &Self) -> Self {
        Self {
            left: self.left.min(other.left),
            top: self.top.min(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
            deepest: self.deepest.max(other.deepest),
        }
    }

    fn padded(&self) -> Self {
        Self {
            left: self.left - BOX_MARGIN,
            top: self.top - BOX_MARGIN,
            right: self.right + BOX_MARGIN,
            bottom: self.bottom + BOX_MARGIN,
            deepest: self.deepest,
        }
    }
}

fn column(index: usize) -> f64 {
    f64::from(u32::try_from(index).unwrap_or(u32::MAX))
}

fn crossing(start: f64, delta: f64, low: f64, high: f64) -> Option<(f64, f64)> {
    if delta == 0.0 {
        return (start >= low && start <= high).then_some((f64::NEG_INFINITY, f64::INFINITY));
    }
    let (a, b) = ((low - start) / delta, (high - start) / delta);
    Some((a.min(b), a.max(b)))
}
