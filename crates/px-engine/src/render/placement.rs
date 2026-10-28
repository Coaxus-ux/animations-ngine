use px_expr::Env;

use super::RenderError;
use super::frame::object_seed;
use super::report::FrameReport;
use super::transform::Transform;
use super::values::Values;
use crate::model::Instance;

/// Frame numbers this close to a whole number are that number, so speeds and offsets
/// don't leave `hold`/`on2` one frame early from rounding.
const FRAME_SNAP: f64 = 1e-6;

/// Where and when one instance is this frame: local coordinates map through `transform`,
/// and its objects are evaluated with `env` (its local time).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Placed {
    pub(crate) transform: Transform,
    pub(crate) env: Env,
    pub(crate) visible: bool,
}

impl Placed {
    pub(crate) const fn root(env: Env) -> Self {
        Self {
            transform: Transform::IDENTITY,
            env,
            visible: true,
        }
    }
}

/// Places every instance, parents first, into `placed`.
pub(crate) fn place(
    instances: &[Instance],
    root: Placed,
    scene_seed: u64,
    placed: &mut [Placed],
    report: &mut FrameReport,
) -> Result<(), RenderError> {
    for index in 0..instances.len().min(placed.len()) {
        let instance = &instances[index];
        let (parent, parent_seed) = match instance.parent {
            Some(parent) => (placed[parent], instances[parent].seed),
            None => (root, scene_seed),
        };
        placed[index] = place_one(instance, &parent, parent_seed, report)?;
    }
    Ok(())
}

fn place_one(
    instance: &Instance,
    parent: &Placed,
    parent_seed: u64,
    report: &mut FrameReport,
) -> Result<Placed, RenderError> {
    let hidden = Placed {
        visible: false,
        ..*parent
    };
    let window = instance.window;
    // A looping host repeats its timeline, so frame `duration` places like frame 0.
    let host_time = parent
        .env
        .loop_period
        .filter(|period| *period > 0.0)
        .map_or(parent.env.time, |period| parent.env.time.rem_euclid(period));
    if !parent.visible || host_time < window.start || host_time >= window.end {
        return Ok(hidden);
    }
    let mut values = Values {
        env: Env {
            index: f64::from(instance.index),
            count: f64::from(instance.count),
            seed: object_seed(parent_seed, usize::MAX),
            ..parent.env
        },
        report,
        object: usize::MAX,
        transform: Transform::IDENTITY,
    };
    let local = Transform {
        scale: values.of(&instance.scale)?,
        flip: instance.flip,
        dx: values.of(&instance.x)?,
        dy: values.of(&instance.y)?,
    };
    let time = local_time(instance, host_time, parent.env.fps);
    let frame = time * parent.env.fps;
    let rounded = libm::round(frame);
    Ok(Placed {
        transform: local.then(&parent.transform),
        env: Env {
            time,
            frame: if (frame - rounded).abs() < FRAME_SNAP {
                rounded
            } else {
                frame
            },
            duration: instance.duration,
            loop_period: instance.looping.then_some(instance.duration),
            ..values.env
        },
        visible: true,
    })
}

fn local_time(instance: &Instance, host_time: f64, fps: f64) -> f64 {
    let window = instance.window;
    let time = window.offset + (host_time - window.start) * window.speed;
    let duration = instance.duration;
    if duration <= 0.0 {
        return time.max(0.0);
    }
    if instance.looping {
        time.rem_euclid(duration)
    } else {
        time.clamp(0.0, (duration - 1.0 / fps.max(1.0)).max(0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::{Instance, local_time};
    use crate::model::Window;

    fn instance(looping: bool) -> Instance {
        Instance {
            window: Window {
                start: 1.0,
                end: 5.0,
                offset: 0.5,
                speed: 2.0,
            },
            duration: 2.0,
            looping,
            ..Instance::default()
        }
    }

    #[test]
    fn local_time_starts_at_the_offset_and_runs_at_the_speed() {
        assert_eq!(local_time(&instance(true), 1.0, 12.0), 0.5);
        assert_eq!(local_time(&instance(true), 1.5, 12.0), 1.5);
        assert_eq!(local_time(&instance(true), 2.0, 12.0), 0.5, "wraps at 2s");
    }

    #[test]
    fn instances_that_do_not_loop_hold_their_last_frame() {
        let last = 2.0 - 1.0 / 12.0;
        assert_eq!(local_time(&instance(false), 4.0, 12.0), last);
    }
}
