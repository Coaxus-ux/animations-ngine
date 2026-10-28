use px_core::Paint;

use super::param::Param;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bounds {
    pub x: Param,
    pub y: Param,
    pub width: Param,
    pub height: Param,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Segment {
    pub x1: Param,
    pub y1: Param,
    pub x2: Param,
    pub y2: Param,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Particles {
    pub area: Bounds,
    pub count: u16,
    pub velocity_x: Param,
    pub velocity_y: Param,
    pub gravity: Param,
    pub life: Param,
    pub size: Param,
    pub wind: Param,
    /// Seconds of travel drawn as a trail behind each particle (rain, sparks).
    pub streak: Param,
    pub fade: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Geometry {
    Rect(Bounds),
    Ellipse(Bounds),
    RoundedRect {
        bounds: Bounds,
        radius: Param,
    },
    Line {
        segment: Segment,
        width: Param,
    },
    Polygon {
        x: Param,
        y: Param,
        points: Vec<[f64; 2]>,
    },
    Blob {
        x: Param,
        y: Param,
        circles: Vec<[f64; 3]>,
        smooth: Param,
    },
    Rope {
        segment: Segment,
        sag: Param,
        sway: Param,
        width: Param,
    },
    Particles(Box<Particles>),
    Shade {
        bounds: Bounds,
        steps: Param,
        round: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Shading {
    Flat,
    Dome { steps: u8, soft: bool },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outline {
    None,
    Full(Paint),
    Selective(Paint),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Texture {
    pub amount: f64,
    pub scale: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GradientDirection {
    Down,
    Up,
    Left,
    Right,
    Radial,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Gradient {
    pub direction: GradientDirection,
    pub steps: i8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub opacity: Param,
    pub tone_shift: Param,
    pub visible: Param,
    pub contact_shadow: Param,
    pub emissive: bool,
    pub casts_shadow: bool,
    pub cleanup: bool,
    pub shading: Shading,
    pub outline: Outline,
    pub texture: Option<Texture>,
    pub gradient: Option<Gradient>,
    pub layer: usize,
    pub clips: Vec<usize>,
    pub instance: Option<usize>,
    /// Root of the shape's randomness; when `None`, derived from the scene seed and the
    /// shape's index.
    pub seed: Option<u64>,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            opacity: Param::Fixed(1.0),
            tone_shift: Param::Fixed(0.0),
            visible: Param::Fixed(1.0),
            contact_shadow: Param::Fixed(0.0),
            emissive: false,
            casts_shadow: false,
            cleanup: false,
            shading: Shading::Flat,
            outline: Outline::None,
            texture: None,
            gradient: None,
            layer: 0,
            clips: Vec::new(),
            instance: None,
            seed: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    pub geometry: Geometry,
    pub paint: Paint,
    pub style: Style,
}
