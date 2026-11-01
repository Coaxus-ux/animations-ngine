#![allow(dead_code, reason = "each test binary uses a different subset")]

use px_core::{FrameCount, FrameRate, LightColor, Paint, Palette, Rgb, Size, Tone};
use px_engine::{
    Bounds, Environment, Geometry, Looping, Param, Scene, SceneSpec, Shape, Style, Timing,
};

pub struct Kit {
    pub palette: Palette,
    pub blank: Paint,
    pub ink: Paint,
    pub mid: Paint,
    pub warm: LightColor,
}

pub fn kit() -> Kit {
    let grays: Vec<Rgb> = (0..5)
        .map(|step| Rgb::new(step * 60, step * 60, step * 60))
        .collect();
    let reds: Vec<Rgb> = (0..5)
        .map(|step| Rgb::new(step * 60, step * 30, 0))
        .collect();
    let mut palette = Palette::new();
    let ramp = palette.push_ramp(&grays).unwrap();
    let twin = palette.push_ramp(&reds).unwrap();
    let warm = LightColor::new(0).unwrap();
    palette.set_twin(ramp, warm, twin).unwrap();
    let paint = |tone| Paint {
        ramp,
        tone: Tone::new(tone),
    };
    Kit {
        palette,
        blank: paint(0),
        ink: paint(1),
        mid: paint(2),
        warm,
    }
}

pub fn expr(source: &str) -> Param {
    Param::Expr(px_expr::compile(source).unwrap())
}

pub fn fixed(value: f64) -> Param {
    Param::Fixed(value)
}

pub fn bounds(x: f64, y: f64, width: f64, height: f64) -> Bounds {
    Bounds {
        x: fixed(x),
        y: fixed(y),
        width: fixed(width),
        height: fixed(height),
    }
}

pub fn shape(geometry: Geometry, paint: Paint) -> Shape {
    Shape {
        geometry,
        paint,
        style: Style::default(),
    }
}

pub fn rect(x: Param, y: Param, width: f64, height: f64, paint: Paint) -> Shape {
    let bounds = Bounds {
        x,
        y,
        width: fixed(width),
        height: fixed(height),
    };
    shape(Geometry::Rect(bounds), paint)
}

pub fn spec(
    kit: &Kit,
    size: (u16, u16),
    frames: u32,
    shapes: Vec<Shape>,
    environment: Environment,
) -> SceneSpec {
    SceneSpec {
        name: "test".to_owned(),
        size: Size::new(size.0, size.1).unwrap(),
        timing: Timing {
            fps: FrameRate::new(12).unwrap(),
            duration: FrameCount::new(frames).unwrap(),
            looping: Looping::Loop,
        },
        seed: 0,
        palette: kit.palette.clone(),
        background: kit.blank,
        shapes,
        environment,
    }
}

pub fn world(
    kit: &Kit,
    size: (u16, u16),
    frames: u32,
    shapes: Vec<Shape>,
    environment: Environment,
) -> Scene {
    Scene::new(spec(kit, size, frames, shapes, environment)).unwrap()
}

pub fn scene(kit: &Kit, size: (u16, u16), frames: u32, shapes: Vec<Shape>) -> Scene {
    world(kit, size, frames, shapes, Environment::default())
}

pub fn render(scene: &Scene, frame: u32) -> Vec<Paint> {
    let mut buffer = px_engine::FrameBuffer::new(scene);
    buffer
        .render(scene, px_core::Frame::new(frame))
        .unwrap()
        .cells()
        .to_vec()
}
