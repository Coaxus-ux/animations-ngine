use std::fmt;

use px_core::{FrameCount, FrameRate, Paint, Palette, PaletteError, Size};
use px_expr::{Env, EvalError, Program};

pub const MAX_SHAPES: usize = 4096;

#[derive(Clone, Debug, PartialEq)]
pub enum Param {
    Fixed(f64),
    Expr(Program),
}

impl Param {
    pub fn value(&self, env: &Env) -> Result<f64, EvalError> {
        match self {
            Self::Fixed(value) => Ok(*value),
            Self::Expr(program) => program.eval(env),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bounds {
    pub x: Param,
    pub y: Param,
    pub width: Param,
    pub height: Param,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    pub x1: Param,
    pub y1: Param,
    pub x2: Param,
    pub y2: Param,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Geometry {
    Rect(Bounds),
    Ellipse(Bounds),
    Line(Segment),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    pub geometry: Geometry,
    pub paint: Paint,
}

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
    pub palette: Palette,
    pub background: Paint,
    pub shapes: Vec<Shape>,
}

#[derive(Clone, Debug)]
pub struct Scene {
    spec: SceneSpec,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ModelError {
    TooManyShapes { count: usize },
    InvalidBackground(PaletteError),
    InvalidShapePaint { shape: usize, error: PaletteError },
}

impl Scene {
    pub fn new(spec: SceneSpec) -> Result<Self, ModelError> {
        if spec.shapes.len() > MAX_SHAPES {
            return Err(ModelError::TooManyShapes {
                count: spec.shapes.len(),
            });
        }
        spec.palette
            .resolve(spec.background)
            .map_err(ModelError::InvalidBackground)?;
        for (shape, item) in spec.shapes.iter().enumerate() {
            spec.palette
                .resolve(item.paint)
                .map_err(|error| ModelError::InvalidShapePaint { shape, error })?;
        }
        Ok(Self { spec })
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
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyShapes { count } => {
                write!(f, "scene has {count} shapes, max is {MAX_SHAPES}")
            }
            Self::InvalidBackground(error) => write!(f, "invalid background: {error}"),
            Self::InvalidShapePaint { shape, error } => {
                write!(f, "shape #{shape} has an invalid paint: {error}")
            }
        }
    }
}

impl std::error::Error for ModelError {}
