use px_core::{LightColor, Paint, RampId};
use px_expr::Wind;

use super::instance::Instance;
use super::param::Param;
use super::shape::Bounds;

#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub parallax: f64,
    pub fog: Param,
    pub fog_paint: Option<Paint>,
}

impl Default for Layer {
    fn default() -> Self {
        Self {
            parallax: 1.0,
            fog: Param::Fixed(0.0),
            fog_paint: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Clip {
    pub bounds: Bounds,
    pub round: bool,
    pub layer: usize,
    pub instance: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Camera {
    pub x: Param,
    pub y: Param,
    /// Scale about the canvas center; shapes are rasterized again at every zoom.
    pub zoom: Param,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            x: Param::Fixed(0.0),
            y: Param::Fixed(0.0),
            zoom: Param::Fixed(1.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum LightKind {
    Point,
    Cone { angle: Param, spread: Param },
    Rect { width: Param, height: Param },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Light {
    pub kind: LightKind,
    pub x: Param,
    pub y: Param,
    pub reach: Param,
    pub intensity: Param,
    pub color: Option<LightColor>,
    pub shadows: bool,
    pub layer: usize,
    pub instance: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ambient {
    pub darkness: Param,
    pub occlusion: f64,
    pub key_light: f64,
}

impl Default for Ambient {
    fn default() -> Self {
        Self {
            darkness: Param::Fixed(0.0),
            occlusion: 0.0,
            key_light: 225.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Reflection {
    pub bounds: Bounds,
    pub axis: Param,
    pub ripple: Param,
    pub speed: Param,
    pub tone_shift: Param,
    pub opacity: Param,
    /// Only pixels of this ramp (the water) receive the reflection.
    pub surface: Option<RampId>,
    pub instance: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Environment {
    pub layers: Vec<Layer>,
    pub clips: Vec<Clip>,
    pub camera: Camera,
    pub lights: Vec<Light>,
    pub ambient: Ambient,
    pub reflections: Vec<Reflection>,
    pub wind: Wind,
    /// Parents come before their children.
    pub instances: Vec<Instance>,
}
