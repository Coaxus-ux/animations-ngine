use alloc::vec::Vec;

use crate::ease::Ease;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Key {
    pub time: f64,
    pub value: f64,
    pub ease: Ease,
    pub hold: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrackLoop {
    None,
    Repeat,
    PingPong,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    keys: Vec<Key>,
    looping: TrackLoop,
}

fn wrap(value: f64, period: f64) -> f64 {
    let rest = value % period;
    if rest < 0.0 { rest + period } else { rest }
}

impl Track {
    #[must_use]
    pub fn new(mut keys: Vec<Key>, looping: TrackLoop) -> Option<Self> {
        if keys.is_empty()
            || keys
                .iter()
                .any(|key| !key.time.is_finite() || !key.value.is_finite())
        {
            return None;
        }
        keys.sort_by(|a, b| a.time.total_cmp(&b.time));
        Some(Self { keys, looping })
    }

    pub(crate) fn sample(&self, time: f64, fps: f64) -> f64 {
        let (first, last) = (self.keys[0], self.keys[self.keys.len() - 1]);
        let span = last.time - first.time;
        let time = match self.looping {
            TrackLoop::Repeat if span > 0.0 => first.time + wrap(time - first.time, span),
            TrackLoop::PingPong if span > 0.0 => {
                let phase = wrap(time - first.time, 2.0 * span);
                first.time
                    + if phase <= span {
                        phase
                    } else {
                        2.0 * span - phase
                    }
            }
            _ => time,
        };
        if time <= first.time {
            return first.value;
        }
        if time >= last.time {
            return last.value;
        }
        let index = self
            .keys
            .partition_point(|key| key.time <= time)
            .saturating_sub(1);
        let (from, to) = (
            self.keys[index],
            self.keys[(index + 1).min(self.keys.len() - 1)],
        );
        let time = if from.hold > 1 && fps > 0.0 {
            let step = f64::from(from.hold) / fps;
            from.time + px_core::floor((time - from.time) / step) * step
        } else {
            time
        };
        let length = to.time - from.time;
        let progress = if length > 0.0 {
            (time - from.time) / length
        } else {
            1.0
        };
        from.value + (to.value - from.value) * from.ease.apply(progress)
    }
}
