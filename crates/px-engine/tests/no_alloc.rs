#![allow(
    unsafe_code,
    reason = "test-only counting allocator, see DECISIONS.md D-002"
)]
#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;
#[path = "support/counting_alloc.rs"]
mod counting_alloc;

use common::{Kit, bounds, expr, fixed, kit, rect, shape, world};
use counting_alloc::assert_no_heap_activity;
use px_core::{Frame, Paint};
use px_engine::{
    Ambient, Anchor, Bounds, Camera, Clip, Detail, Environment, FrameBuffer, Geometry, Gradient,
    GradientDirection, Instance, Layer, Light, LightKind, Outline, Particles, Reflection, Segment,
    Shading, Shape, Style, Text, TextCell, Texture, Window,
};
use px_expr::Wind;

fn styled(geometry: Geometry, paint: Paint, style: Style) -> Shape {
    Shape {
        geometry,
        paint,
        style,
    }
}

fn lit(kit: &Kit) -> Style {
    Style {
        shading: Shading::Dome {
            steps: 2,
            soft: true,
        },
        outline: Outline::Selective(kit.blank),
        texture: Some(Texture {
            amount: 0.4,
            scale: 3.0,
        }),
        casts_shadow: true,
        cleanup: true,
        contact_shadow: fixed(1.0),
        ..Style::default()
    }
}

fn shapes(kit: &Kit) -> Vec<Shape> {
    let lit = lit(kit);
    let segment = Segment {
        x1: fixed(2.0),
        y1: fixed(2.0),
        x2: expr("20 + sin(t * tau) * 6"),
        y2: fixed(28.0),
    };
    vec![
        rect(expr("floor(t * 4)"), fixed(1.0), 5.0, 5.0, kit.ink),
        styled(
            Geometry::Ellipse(Bounds {
                x: expr("10 + sin(t * tau) * 8"),
                ..bounds(0.0, 4.0, 12.0, 10.0)
            }),
            kit.mid,
            lit.clone(),
        ),
        styled(
            Geometry::RoundedRect {
                bounds: bounds(14.0, 14.0, 12.0, 8.0),
                radius: fixed(3.0),
            },
            kit.mid,
            Style {
                gradient: Some(Gradient {
                    direction: GradientDirection::Radial,
                    steps: 2,
                }),
                clips: vec![0],
                ..lit.clone()
            },
        ),
        shape(
            Geometry::Line {
                segment: segment.clone(),
                width: fixed(3.0),
            },
            kit.ink,
        ),
        shape(
            Geometry::Polygon {
                x: fixed(4.0),
                y: fixed(20.0),
                points: vec![[0.0, 0.0], [8.0, 0.0], [4.0, 7.0]],
            },
            kit.ink,
        ),
        styled(
            Geometry::Blob {
                x: fixed(20.0),
                y: fixed(4.0),
                circles: vec![[0.0, 0.0, 3.0], [4.0, 1.0, 3.0]],
                smooth: fixed(2.0),
            },
            kit.mid,
            Style {
                layer: 1,
                emissive: true,
                ..Style::default()
            },
        ),
        shape(
            Geometry::Rope {
                segment,
                sag: fixed(4.0),
                sway: expr("sin(t * tau)"),
                width: fixed(1.0),
            },
            kit.ink,
        ),
        shape(
            Geometry::Particles(Box::new(Particles {
                area: bounds(0.0, 0.0, 32.0, 4.0),
                count: 40,
                velocity_x: fixed(1.0),
                velocity_y: fixed(6.0),
                gravity: fixed(2.0),
                life: fixed(2.0),
                size: fixed(1.0),
                wind: fixed(1.0),
                streak: fixed(0.1),
                fade: true,
            })),
            kit.mid,
        ),
        text(kit),
        shape(
            Geometry::Shade {
                bounds: bounds(0.0, 24.0, 32.0, 8.0),
                steps: fixed(-1.0),
                round: true,
            },
            kit.blank,
        ),
    ]
}

fn text(kit: &Kit) -> Shape {
    shape(
        Geometry::Text(Box::new(Text {
            x: fixed(4.0),
            y: fixed(4.0),
            size: expr("2 + t"),
            reveal: expr("t * 8"),
            scatter: expr("max(0, 6 - t * 12)"),
            cells: (0..12)
                .map(|i| TextCell {
                    x: i,
                    y: i % 3,
                    glyph: i.unsigned_abs() / 2,
                    scale: 1 + u8::from(i > 8),
                })
                .collect(),
        })),
        kit.ink,
    )
}

fn environment(kit: &Kit) -> Environment {
    Environment {
        layers: vec![
            Layer::default(),
            Layer {
                parallax: 0.5,
                fog: fixed(0.5),
                fog_paint: Some(kit.mid),
            },
        ],
        clips: vec![Clip {
            bounds: bounds(12.0, 12.0, 10.0, 10.0),
            round: true,
            layer: 0,
            instance: None,
        }],
        camera: Camera {
            x: expr("t * 2"),
            y: fixed(0.0),
            zoom: expr("1 + t * 0.25"),
        },
        lights: vec![
            Light {
                kind: LightKind::Point,
                x: fixed(16.0),
                y: fixed(16.0),
                reach: fixed(20.0),
                intensity: fixed(1.0),
                color: Some(kit.warm),
                shadows: true,
                layer: 0,
                instance: None,
                anchor: None,
            },
            Light {
                kind: LightKind::Cone {
                    angle: fixed(90.0),
                    spread: fixed(40.0),
                },
                x: fixed(4.0),
                y: fixed(0.0),
                reach: fixed(30.0),
                intensity: fixed(1.0),
                color: None,
                shadows: false,
                layer: 0,
                instance: None,
                anchor: None,
            },
            Light {
                kind: LightKind::Rect {
                    width: fixed(6.0),
                    height: fixed(4.0),
                },
                x: fixed(24.0),
                y: fixed(24.0),
                reach: fixed(8.0),
                intensity: fixed(1.0),
                color: None,
                shadows: false,
                layer: 0,
                instance: None,
                anchor: None,
            },
        ],
        ambient: Ambient {
            darkness: fixed(1.0),
            occlusion: 1.0,
            key_light: 225.0,
        },
        reflections: vec![Reflection {
            bounds: bounds(0.0, 26.0, 32.0, 6.0),
            axis: fixed(26.0),
            ripple: fixed(1.0),
            speed: fixed(2.0),
            tone_shift: fixed(-1.0),
            opacity: fixed(0.8),
            surface: None,
            instance: None,
        }],
        wind: Wind {
            strength: 1.0,
            gust: 0.5,
            speed: 16.0,
        },
        instances: instances(),
    }
}

fn instances() -> Vec<Instance> {
    vec![
        Instance {
            x: expr("4 + t"),
            scale: expr("1 + 0.5 * sin(t * tau)"),
            flip: true,
            window: Window {
                start: 0.25,
                end: 3.0,
                offset: 0.1,
                speed: 1.5,
            },
            duration: 1.0,
            ..Instance::default()
        },
        Instance {
            parent: Some(0),
            y: fixed(2.0),
            looping: false,
            duration: 0.5,
            ..Instance::default()
        },
    ]
}

#[test]
fn rendering_performs_no_heap_activity() {
    let kit = kit();
    let mut shapes = shapes(&kit);
    shapes[0].style.instance = Some(1);
    shapes[1].style.instance = Some(0);
    let mut environment = environment(&kit);
    environment.lights[1].instance = Some(0);
    environment.lights[0].anchor = Some(Anchor {
        instance: Some(1),
        x: expr("2 + t"),
        y: fixed(1.0),
    });
    shapes[2].style.anchor = Some(Anchor {
        instance: Some(0),
        x: fixed(3.0),
        y: expr("t"),
    });
    let scene = world(&kit, (32, 32), 24, shapes, environment);
    let detailed = scene
        .clone()
        .with_detail(Detail::new(2.5).unwrap())
        .unwrap();
    let mut buffer = FrameBuffer::new(&scene);
    let mut detailed_buffer = FrameBuffer::new(&detailed);

    assert_no_heap_activity(|| {
        for frame in 0..48 {
            buffer.render(&scene, Frame::new(frame)).unwrap();
            detailed_buffer
                .render(&detailed, Frame::new(frame))
                .unwrap();
        }
    });
}
