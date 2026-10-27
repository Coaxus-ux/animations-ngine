use std::fmt;

use px_core::{FrameCount, FrameRate, Paint, Palette, PaletteError, Size};
use px_expr::{Env, EvalError, Program};

/// Maximum number of shapes in one scene; bounds the work of a frame.
pub const MAX_SHAPES: usize = 4096;

/// A numeric parameter: fixed, or an expression evaluated every frame.
#[derive(Clone, Debug, PartialEq)]
pub enum Param {
    /// A constant value.
    Fixed(f64),
    /// A compiled expression.
    Expr(Program),
}

impl Param {
    /// The value at `env`.
    ///
    /// # Errors
    ///
    /// Propagates [`EvalError`] from expressions.
    pub fn value(&self, env: &Env) -> Result<f64, EvalError> {
        match self {
            Self::Fixed(value) => Ok(*value),
            Self::Expr(program) => program.eval(env),
        }
    }
}

/// An axis-aligned box: top-left corner and size.
#[derive(Clone, Debug, PartialEq)]
pub struct Bounds {
    /// Left edge.
    pub x: Param,
    /// Top edge.
    pub y: Param,
    /// Width.
    pub width: Param,
    /// Height.
    pub height: Param,
}

/// A line segment between two points.
#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    /// Start column.
    pub x1: Param,
    /// Start row.
    pub y1: Param,
    /// End column.
    pub x2: Param,
    /// End row.
    pub y2: Param,
}

/// The geometry of a shape.
#[derive(Clone, Debug, PartialEq)]
pub enum Geometry {
    /// A filled rectangle.
    Rect(Bounds),
    /// A filled ellipse inscribed in its bounds.
    Ellipse(Bounds),
    /// A one-pixel line.
    Line(Segment),
}

/// A drawable shape.
#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    /// What to draw.
    pub geometry: Geometry,
    /// The paint it uses.
    pub paint: Paint,
}

/// Whether the scene is meant to loop seamlessly.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Looping {
    /// Plays once.
    Once,
    /// Loops: the frame after the last must equal the first.
    Loop,
}

/// Frame rate, length and looping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Timing {
    /// Frames per second.
    pub fps: FrameRate,
    /// Number of frames.
    pub duration: FrameCount,
    /// Looping behavior.
    pub looping: Looping,
}

/// Everything needed to build a [`Scene`]; validated by [`Scene::new`].
#[derive(Clone, Debug)]
pub struct SceneSpec {
    /// The scene's name.
    pub name: String,
    /// Canvas size.
    pub size: Size,
    /// Timing.
    pub timing: Timing,
    /// The palette every paint refers to.
    pub palette: Palette,
    /// Paint of the empty canvas.
    pub background: Paint,
    /// Shapes, drawn in order (later shapes on top).
    pub shapes: Vec<Shape>,
}

/// A validated, immutable scene ready to render.
#[derive(Clone, Debug)]
pub struct Scene {
    spec: SceneSpec,
}

/// Why a scene spec is invalid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ModelError {
    /// More than [`MAX_SHAPES`] shapes.
    TooManyShapes {
        /// Shapes given.
        count: usize,
    },
    /// The background paint does not exist in the palette.
    InvalidBackground(PaletteError),
    /// A shape's paint does not exist in the palette.
    InvalidShapePaint {
        /// Index of the shape.
        shape: usize,
        /// The palette error.
        error: PaletteError,
    },
}

impl Scene {
    /// Validates a spec: shape count and every paint against the palette.
    ///
    /// # Errors
    ///
    /// A [`ModelError`] describing the first problem.
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

    /// The scene's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.spec.name
    }

    /// Canvas size.
    #[must_use]
    pub const fn size(&self) -> Size {
        self.spec.size
    }

    /// Timing.
    #[must_use]
    pub const fn timing(&self) -> Timing {
        self.spec.timing
    }

    /// The palette.
    #[must_use]
    pub const fn palette(&self) -> &Palette {
        &self.spec.palette
    }

    /// Background paint.
    #[must_use]
    pub const fn background(&self) -> Paint {
        self.spec.background
    }

    /// Shapes in draw order.
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
