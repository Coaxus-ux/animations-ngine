use core::f64::consts::PI;

use px_core::{Frame, hash_seed, snap_coord, snap_extent};
use px_expr::Env;
use px_raster::Rect;

use super::RenderError;
use super::composite::{Evaluated, composite};
use super::geometry::rasterize;
use super::lighting::{self, Lit};
use super::mask::Mask;
use super::reflection::{Mirror, reflect};
use super::report::FrameReport;
use super::surface::{Region, Surface};
use super::values::Values;
use crate::model::{
    Bounds, Geometry, LightKind, Looping, MAX_CLIPS, MAX_LAYERS, MAX_LIGHTS, Param, Scene,
};

const MAX_NESTED_CLIPS: usize = 8;

#[must_use]
pub fn object_seed(scene_seed: u64, index: usize) -> u64 {
    hash_seed(scene_seed, &[u64::try_from(index).unwrap_or(u64::MAX)])
}

pub(crate) struct Pass<'s, 'r> {
    pub(crate) scene: &'s Scene,
    pub(crate) env: Env,
    pub(crate) report: &'r mut FrameReport,
}

impl Pass<'_, '_> {
    fn values(&mut self, object: usize, offset: [f64; 2]) -> Values<'_> {
        let env = Env {
            seed: object_seed(self.scene.seed(), object),
            ..self.env
        };
        Values {
            env,
            report: self.report,
            object,
            offset,
        }
    }

    fn eval(&mut self, param: &Param) -> Result<f64, RenderError> {
        self.values(usize::MAX, [0.0, 0.0]).of(param)
    }
}

pub(crate) fn render(
    scene: &Scene,
    frame: Frame,
    surface: &mut Surface<'_>,
    mask: &mut Mask<'_>,
    report: &mut FrameReport,
) -> Result<(), RenderError> {
    surface.cells.fill(scene.background());
    surface.depth.fill(0);
    surface.flags.fill(0);
    let mut pass = Pass {
        scene,
        env: env_at(scene, frame),
        report,
    };
    let world = scene.environment();
    let camera = [pass.eval(&world.camera.x)?, pass.eval(&world.camera.y)?];
    let mut offsets = [[0.0; 2]; MAX_LAYERS];
    for (offset, layer) in offsets.iter_mut().zip(&world.layers) {
        *offset = [-camera[0] * layer.parallax, -camera[1] * layer.parallax];
    }
    let layer_offset = |layer: usize| {
        offsets
            .get(layer)
            .copied()
            .unwrap_or([-camera[0], -camera[1]])
    };
    let mut regions = [Region::default(); MAX_CLIPS];
    for (region, clip) in regions.iter_mut().zip(&world.clips) {
        let mut values = pass.values(usize::MAX, layer_offset(clip.layer));
        *region = Region {
            rect: rect_of(&clip.bounds, &mut values)?,
            round: clip.round,
        };
    }
    let light_direction = key_light(world.ambient.key_light);
    for (index, shape) in scene.shapes().iter().enumerate() {
        let style = &shape.style;
        let mut values = pass.values(index, layer_offset(style.layer));
        if values.of(&style.visible)? <= 0.5 {
            continue;
        }
        let layer = world.layers.get(style.layer);
        let fog = match layer {
            Some(layer) => values.of(&layer.fog)?,
            None => 0.0,
        };
        let shade_steps = match &shape.geometry {
            Geometry::Shade { steps, .. } => values.of(steps)?,
            _ => 0.0,
        };
        let mut clips = [Region::default(); MAX_NESTED_CLIPS];
        let used = style.clips.len().min(MAX_NESTED_CLIPS);
        for (target, clip) in clips.iter_mut().zip(&style.clips) {
            *target = regions.get(*clip).copied().unwrap_or_default();
        }
        let evaluated = Evaluated {
            depth: u16::try_from(index + 1).unwrap_or(u16::MAX),
            seed: values.env.seed,
            opacity: values.of(&style.opacity)?,
            tone_shift: values.of(&style.tone_shift)?,
            contact_shadow: values.of(&style.contact_shadow)?,
            shade_steps,
            fog,
            fog_paint: layer.and_then(|layer| layer.fog_paint),
            clips: &clips[..used],
            light: light_direction,
        };
        mask.clear();
        rasterize(&shape.geometry, &mut values, mask)?;
        composite(shape, &evaluated, scene.palette(), mask, surface);
    }
    light_scene(&mut pass, surface, layer_offset(0))?;
    for reflection in &world.reflections {
        let mut values = pass.values(usize::MAX, layer_offset(0));
        let (env, speed) = (values.env, values.of(&reflection.speed)?);
        let mirror = Mirror {
            rect: rect_of(&reflection.bounds, &mut values)?,
            axis: values.y(&reflection.axis)?,
            ripple: values.of(&reflection.ripple)?,
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
    offset: [f64; 2],
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
    for (lit, lamp) in lits.iter_mut().zip(&world.lights) {
        let mut values = pass.values(usize::MAX, offset);
        let (x, y) = (values.x(&lamp.x)?, values.y(&lamp.y)?);
        let shape = match &lamp.kind {
            LightKind::Point => lighting::Shape::Point,
            LightKind::Cone { angle, spread } => lighting::Shape::Cone {
                direction: values.of(angle)? * PI / 180.0,
                half_spread: values.of(spread)? * PI / 360.0,
            },
            LightKind::Rect { width, height } => lighting::Shape::Rect(Rect {
                x: snap_coord(x),
                y: snap_coord(y),
                width: snap_extent(values.of(width)?),
                height: snap_extent(values.of(height)?),
            }),
        };
        *lit = Lit {
            shape,
            x,
            y,
            reach: values.of(&lamp.reach)?,
            intensity: values.of(&lamp.intensity)?,
            color: lamp.color,
            shadows: lamp.shadows,
        };
    }
    let count = world.lights.len().min(MAX_LIGHTS);
    lighting::light(surface, pass.scene.palette(), &ambient, &lits[..count]);
    Ok(())
}

fn rect_of(bounds: &Bounds, values: &mut Values<'_>) -> Result<Rect, RenderError> {
    Ok(Rect {
        x: snap_coord(values.x(&bounds.x)?),
        y: snap_coord(values.y(&bounds.y)?),
        width: snap_extent(values.of(&bounds.width)?),
        height: snap_extent(values.of(&bounds.height)?),
    })
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
