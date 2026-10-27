use px_core::{FrameCount, FrameRate, Paint, Palette, Rgb, Size, Tone};
use px_engine::{Bounds, Geometry, Looping, Param, Scene, SceneSpec, Shape, Timing};

pub struct Kit {
    pub palette: Palette,
    pub blank: Paint,
    pub ink: Paint,
}

pub fn kit() -> Kit {
    let mut palette = Palette::new();
    let ramp = palette
        .push_ramp(&[Rgb::new(0, 0, 0), Rgb::new(255, 255, 255)])
        .unwrap();
    Kit {
        palette,
        blank: Paint {
            ramp,
            tone: Tone::new(0),
        },
        ink: Paint {
            ramp,
            tone: Tone::new(1),
        },
    }
}

pub fn expr(source: &str) -> Param {
    Param::Expr(px_expr::compile(source).unwrap())
}

pub fn rect(x: Param, y: Param, width: f64, height: f64, paint: Paint) -> Shape {
    let bounds = Bounds {
        x,
        y,
        width: Param::Fixed(width),
        height: Param::Fixed(height),
    };
    Shape {
        geometry: Geometry::Rect(bounds),
        paint,
    }
}

pub fn scene(kit: &Kit, size: (u16, u16), frames: u32, shapes: Vec<Shape>) -> Scene {
    Scene::new(SceneSpec {
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
    })
    .unwrap()
}
