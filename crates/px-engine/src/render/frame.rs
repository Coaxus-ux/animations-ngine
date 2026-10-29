use core::f64::consts::PI;

use px_core::{Frame, hash_seed};
use px_expr::Env;

use super::RenderError;
use super::draw::{Context, draw_shapes};
use super::geometry::{rect, rect_of};
use super::lighting::{self, Lit};
use super::placement::{Placed, anchored, place};
use super::reflection::{Mirror, reflect};
use super::report::FrameReport;
use super::stamp::Capture;
use super::surface::{Region, Surface, Target};
use super::transform::Transform;
use super::values::Values;
use crate::model::{Anchor, LightKind, Looping, MAX_CLIPS, MAX_LAYERS, MAX_LIGHTS, Param, Scene};

#[must_use]
pub fn object_seed(scene_seed: u64, index: usize) -> u64 {
    hash_seed(scene_seed, &[u64::try_from(index).unwrap_or(u64::MAX)])
}

pub(crate) struct Pass<'s, 'r> {
    pub(crate) scene: &'s Scene,
    pub(crate) root: Placed,
    pub(crate) placed: &'r [Placed],
    pub(crate) layers: [Transform; MAX_LAYERS],
    pub(crate) report: &'r mut FrameReport,
}

impl Pass<'_, '_> {
    /// Values for one object of `owner` (an instance, or the scene itself) drawn in `layer`,
    /// positioned from `anchor` when it is hooked to one, or `None` while hidden.
    pub(super) fn values(
        &mut self,
        object: usize,
        seed: u64,
        owner: Option<usize>,
        layer: usize,
        anchor: Option<&Anchor>,
    ) -> Result<Option<Values<'_>>, RenderError> {
        let placed = owner
            .and_then(|instance| self.placed.get(instance))
            .copied()
            .unwrap_or(self.root);
        if !placed.visible {
            return Ok(None);
        }
        let origin = match anchor {
            Some(anchor) => {
                let seed = self.instance_seed(anchor.instance);
                anchored(anchor, self.placed, &self.root, seed, self.report)?
            }
            None => Some(placed.transform),
        };
        let layer = self.layers.get(layer).copied().unwrap_or(self.layers[0]);
        Ok(origin.map(|origin| Values {
            env: Env { seed, ..placed.env },
            report: &mut *self.report,
            object,
            transform: origin.then(&layer),
            timed: false,
        }))
    }

    fn instance_seed(&self, instance: Option<usize>) -> u64 {
        instance
            .and_then(|instance| self.scene.environment().instances.get(instance))
            .map_or(self.scene.seed(), |instance| instance.seed)
    }

    /// The seed of an object that isn't a shape (a clip, a light, a reflection).
    fn object_seed(&self, owner: Option<usize>) -> u64 {
        object_seed(self.instance_seed(owner), usize::MAX)
    }

    fn eval(&mut self, param: &Param) -> Result<f64, RenderError> {
        Values {
            env: Env {
                seed: object_seed(self.scene.seed(), usize::MAX),
                ..self.root.env
            },
            report: &mut *self.report,
            object: usize::MAX,
            transform: Transform::IDENTITY,
            timed: false,
        }
        .of(param)
    }
}

/// Draws `frame`; with `capture`, also records the stamps of shapes that never change.
pub(crate) fn render(
    scene: &Scene,
    frame: Frame,
    target: Target<'_, '_>,
    report: &mut FrameReport,
    capture: Option<&mut Capture>,
) -> Result<(), RenderError> {
    let Target {
        surface,
        mask,
        placed,
    } = target;
    surface.cells.fill(scene.background());
    surface.depth.fill(0);
    surface.flags.fill(0);
    let world = scene.environment();
    let root = Placed::root(env_at(scene, frame));
    place(&world.instances, root, scene.seed(), placed, report)?;
    let mut pass = Pass {
        scene,
        root,
        placed,
        layers: [Transform::IDENTITY; MAX_LAYERS],
        report,
    };
    let view = View {
        x: pass.eval(&world.camera.x)?,
        y: pass.eval(&world.camera.y)?,
        zoom: pass.eval(&world.camera.zoom)?,
        center: [
            f64::from(surface.width) / 2.0,
            f64::from(surface.height) / 2.0,
        ],
    };
    pass.layers = [view.through(1.0); MAX_LAYERS];
    for (transform, layer) in pass.layers.iter_mut().zip(&world.layers) {
        *transform = view.through(layer.parallax);
    }
    let clips = clip_regions(&mut pass)?;
    let context = Context {
        clips: &clips,
        light: key_light(world.ambient.key_light),
    };
    draw_shapes(&mut pass, &context, mask, surface, capture)?;
    light_scene(&mut pass, surface, mask.distance)?;
    reflect_scene(&mut pass, surface)
}

fn clip_regions(pass: &mut Pass<'_, '_>) -> Result<[Region; MAX_CLIPS], RenderError> {
    let mut regions = [Region::default(); MAX_CLIPS];
    for (region, clip) in regions.iter_mut().zip(&pass.scene.environment().clips) {
        let owner_seed = pass.object_seed(clip.instance);
        if let Some(mut values) =
            pass.values(usize::MAX, owner_seed, clip.instance, clip.layer, None)?
        {
            *region = Region {
                rect: rect(&clip.bounds, &mut values)?,
                round: clip.round,
            };
        }
    }
    Ok(regions)
}

fn reflect_scene(pass: &mut Pass<'_, '_>, surface: &mut Surface<'_>) -> Result<(), RenderError> {
    let scene = pass.scene;
    for reflection in &scene.environment().reflections {
        let owner_seed = pass.object_seed(reflection.instance);
        let Some(mut values) = pass.values(usize::MAX, owner_seed, reflection.instance, 0, None)?
        else {
            continue;
        };
        let (env, speed) = (values.env, values.of(&reflection.speed)?);
        let mirror = Mirror {
            rect: rect(&reflection.bounds, &mut values)?,
            axis: values.y(&reflection.axis)?,
            ripple: values.extent(&reflection.ripple)?,
            phase: loop_phase(&env, speed),
            tone_shift: values.of(&reflection.tone_shift)?,
            opacity: values.of(&reflection.opacity)?,
            surface: reflection.surface,
        };
        reflect(surface, scene.palette(), &mirror);
    }
    Ok(())
}

fn light_scene(
    pass: &mut Pass<'_, '_>,
    surface: &mut Surface<'_>,
    scratch: &mut [u16],
) -> Result<(), RenderError> {
    let world = pass.scene.environment();
    let ambient = lighting::Ambient {
        darkness: pass.eval(&world.ambient.darkness)?,
        occlusion: world.ambient.occlusion,
    };
    let mut lits = [Lit {
        shape: lighting::Shape::Point,
        x: 0.0,
        y: 0.0,
        reach: 0.0,
        intensity: 0.0,
        color: None,
        shadows: false,
    }; MAX_LIGHTS];
    let mut count = 0;
    for lamp in &world.lights {
        let owner_seed = pass.object_seed(lamp.instance);
        let anchor = lamp.anchor.as_ref();
        let Some(mut values) =
            pass.values(usize::MAX, owner_seed, lamp.instance, lamp.layer, anchor)?
        else {
            continue;
        };
        let transform = values.transform;
        let [x, y] = values.point(&lamp.x, &lamp.y)?;
        let shape = match &lamp.kind {
            LightKind::Point => lighting::Shape::Point,
            LightKind::Cone { angle, spread } => lighting::Shape::Cone {
                direction: transform.angle(values.of(angle)? * PI / 180.0),
                half_spread: values.of(spread)? * PI / 360.0,
            },
            LightKind::Rect { width, height } => {
                lighting::Shape::Rect(rect_of([&lamp.x, &lamp.y, width, height], &mut values)?)
            }
        };
        let Some(lit) = lits.get_mut(count) else {
            break;
        };
        count += 1;
        *lit = Lit {
            shape,
            x,
            y,
            reach: values.extent(&lamp.reach)?,
            intensity: values.of(&lamp.intensity)?,
            color: lamp.color,
            shadows: lamp.shadows,
        };
    }
    lighting::light(
        surface,
        pass.scene.palette(),
        &ambient,
        &lits[..count],
        scratch,
    );
    Ok(())
}

/// The camera: a pan scaled by each layer's parallax, then a zoom about the canvas center.
struct View {
    x: f64,
    y: f64,
    zoom: f64,
    center: [f64; 2],
}

impl View {
    fn through(&self, parallax: f64) -> Transform {
        let along =
            |pan: f64, center: f64| -pan * parallax * self.zoom + center * (1.0 - self.zoom);
        Transform {
            scale: self.zoom,
            flip: false,
            dx: along(self.x, self.center[0]),
            dy: along(self.y, self.center[1]),
        }
    }
}

/// `time * speed` radians, with the speed rounded to whole turns per loop so looping
/// scenes have no seam.
fn loop_phase(env: &Env, speed: f64) -> f64 {
    match env.loop_period {
        Some(period) if period > 0.0 => {
            let turns = libm::round(speed * period / (2.0 * PI));
            2.0 * PI * turns * env.time / period
        }
        _ => env.time * speed,
    }
}

fn key_light(degrees: f64) -> [f64; 3] {
    let angle = degrees * PI / 180.0;
    let (x, y, z) = (libm::cos(angle), libm::sin(angle), 0.8);
    let length = libm::sqrt(x * x + y * y + z * z);
    [x / length, y / length, z / length]
}

fn env_at(scene: &Scene, frame: Frame) -> Env {
    let timing = scene.timing();
    let duration = timing.fps.seconds_at(Frame::new(timing.duration.get()));
    Env {
        time: timing.fps.seconds_at(frame),
        frame: f64::from(frame.index()),
        fps: f64::from(timing.fps.get()),
        duration,
        loop_period: (timing.looping == Looping::Loop).then_some(duration),
        wind: scene.environment().wind,
        ..Env::default()
    }
}
