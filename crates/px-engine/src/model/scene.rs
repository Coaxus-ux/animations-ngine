use std::fmt;

use px_core::{FrameCount, FrameRate, Paint, Palette, PaletteError, Size};

use super::shape::{Geometry, Outline, Shape};
use super::world::Environment;

/// Draw order doubles as depth, which is stored as a `u16` with 0 for the background.
pub const MAX_SHAPES: usize = u16::MAX as usize - 1;
pub const MAX_INSTANCES: usize = 4096;
pub const MAX_LAYERS: usize = 16;
pub const MAX_CLIPS: usize = 64;
pub const MAX_LIGHTS: usize = 16;
pub const MAX_REFLECTIONS: usize = 8;
pub const MAX_PARTICLES: u16 = 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Looping {
    Once,
    Loop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Timing {
    pub fps: FrameRate,
    pub duration: FrameCount,
    pub looping: Looping,
}

#[derive(Clone, Debug)]
pub struct SceneSpec {
    pub name: String,
    pub size: Size,
    pub timing: Timing,
    pub seed: u64,
    pub palette: Palette,
    pub background: Paint,
    pub shapes: Vec<Shape>,
    pub environment: Environment,
}

#[derive(Clone, Debug)]
pub struct Scene {
    spec: SceneSpec,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ModelError {
    TooMany {
        what: &'static str,
        count: usize,
        max: usize,
    },
    InvalidBackground(PaletteError),
    InvalidShapePaint {
        shape: usize,
        error: PaletteError,
    },
    UnknownLayer {
        shape: usize,
    },
    UnknownClip {
        shape: usize,
    },
    UnknownInstance {
        what: &'static str,
        index: usize,
    },
}

/// How much a scene asks of the renderer.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Stats {
    pub shapes: usize,
    pub instances: usize,
    pub lights: usize,
    pub layers: usize,
    pub pixels: usize,
}

impl Scene {
    pub fn new(spec: SceneSpec) -> Result<Self, ModelError> {
        let world = &spec.environment;
        let limits = [
            ("shapes", spec.shapes.len(), MAX_SHAPES),
            ("layers", world.layers.len(), MAX_LAYERS),
            ("clips", world.clips.len(), MAX_CLIPS),
            ("lights", world.lights.len(), MAX_LIGHTS),
            ("reflections", world.reflections.len(), MAX_REFLECTIONS),
            ("instances", world.instances.len(), MAX_INSTANCES),
        ];
        for (what, count, max) in limits {
            if count > max {
                return Err(ModelError::TooMany { what, count, max });
            }
        }
        spec.palette
            .resolve(spec.background)
            .map_err(ModelError::InvalidBackground)?;
        for (index, shape) in spec.shapes.iter().enumerate() {
            validate_shape(&spec, index, shape)?;
        }
        validate_instances(world)?;
        Ok(Self { spec })
    }

    #[must_use]
    pub fn stats(&self) -> Stats {
        let world = &self.spec.environment;
        Stats {
            shapes: self.spec.shapes.len(),
            instances: world.instances.len(),
            lights: world.lights.len(),
            layers: world.layers.len().max(1),
            pixels: self.spec.size.cell_count(),
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.spec.name
    }

    #[must_use]
    pub const fn size(&self) -> Size {
        self.spec.size
    }

    #[must_use]
    pub const fn timing(&self) -> Timing {
        self.spec.timing
    }

    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.spec.seed
    }

    #[must_use]
    pub const fn palette(&self) -> &Palette {
        &self.spec.palette
    }

    #[must_use]
    pub const fn background(&self) -> Paint {
        self.spec.background
    }

    #[must_use]
    pub fn shapes(&self) -> &[Shape] {
        &self.spec.shapes
    }

    #[must_use]
    pub const fn environment(&self) -> &Environment {
        &self.spec.environment
    }
}

fn validate_shape(spec: &SceneSpec, index: usize, shape: &Shape) -> Result<(), ModelError> {
    let paint_error = |error| ModelError::InvalidShapePaint {
        shape: index,
        error,
    };
    spec.palette.resolve(shape.paint).map_err(paint_error)?;
    if let Outline::Full(paint) | Outline::Selective(paint) = shape.style.outline {
        spec.palette.resolve(paint).map_err(paint_error)?;
    }
    if shape.style.layer >= spec.environment.layers.len().max(1) {
        return Err(ModelError::UnknownLayer { shape: index });
    }
    if shape
        .style
        .clips
        .iter()
        .any(|clip| *clip >= spec.environment.clips.len())
    {
        return Err(ModelError::UnknownClip { shape: index });
    }
    let style = &shape.style;
    let anchor = style.anchor.as_ref().and_then(|anchor| anchor.instance);
    if [style.instance, anchor]
        .iter()
        .flatten()
        .any(|instance| *instance >= spec.environment.instances.len())
    {
        return Err(ModelError::UnknownInstance {
            what: "shape",
            index,
        });
    }
    if let Geometry::Particles(particles) = &shape.geometry
        && particles.count > MAX_PARTICLES
    {
        let count = usize::from(particles.count);
        return Err(ModelError::TooMany {
            what: "particles",
            count,
            max: usize::from(MAX_PARTICLES),
        });
    }
    Ok(())
}

/// Every instance reference points at an existing instance, and parents come first.
fn validate_instances(world: &Environment) -> Result<(), ModelError> {
    let count = world.instances.len();
    let check = |what: &'static str, owners: Vec<Option<usize>>| match owners
        .iter()
        .position(|owner| owner.is_some_and(|owner| owner >= count))
    {
        Some(index) => Err(ModelError::UnknownInstance { what, index }),
        None => Ok(()),
    };
    check(
        "clip",
        world.clips.iter().map(|clip| clip.instance).collect(),
    )?;
    check(
        "light",
        world.lights.iter().map(|light| light.instance).collect(),
    )?;
    check(
        "light anchor",
        world
            .lights
            .iter()
            .map(|light| light.anchor.as_ref().and_then(|anchor| anchor.instance))
            .collect(),
    )?;
    check(
        "reflection",
        world
            .reflections
            .iter()
            .map(|mirror| mirror.instance)
            .collect(),
    )?;
    match world
        .instances
        .iter()
        .enumerate()
        .position(|(index, instance)| {
            let anchor = instance.anchor.as_ref().and_then(|anchor| anchor.instance);
            [instance.parent, anchor]
                .iter()
                .flatten()
                .any(|earlier| *earlier >= index)
        }) {
        Some(index) => Err(ModelError::UnknownInstance {
            what: "instance",
            index,
        }),
        None => Ok(()),
    }
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooMany { what, count, max } => {
                write!(f, "scene has {count} {what}, max is {max}")
            }
            Self::InvalidBackground(error) => write!(f, "invalid background: {error}"),
            Self::InvalidShapePaint { shape, error } => {
                write!(f, "shape #{shape} has an invalid paint: {error}")
            }
            Self::UnknownLayer { shape } => write!(f, "shape #{shape} is in an unknown layer"),
            Self::UnknownClip { shape } => write!(f, "shape #{shape} uses an unknown clip"),
            Self::UnknownInstance { what, index } => {
                write!(f, "{what} #{index} belongs to an unknown instance")
            }
        }
    }
}

impl std::error::Error for ModelError {}
