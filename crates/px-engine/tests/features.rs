#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use common::{bounds, expr, fixed, kit, render, scene, shape, world};
use px_core::Paint;
use px_engine::{
    Ambient, Clip, Environment, Geometry, Layer, Light, LightKind, Outline, Reflection, Shading,
    Shape, Style,
};

fn count(cells: &[Paint], paint: Paint) -> usize {
    cells.iter().filter(|cell| **cell == paint).count()
}

fn tones(cells: &[Paint]) -> std::collections::BTreeSet<u8> {
    cells.iter().map(|cell| cell.tone.index()).collect()
}

fn styled(geometry: Geometry, paint: Paint, style: Style) -> Shape {
    Shape {
        geometry,
        paint,
        style,
    }
}

fn lamp(x: f64, y: f64, shadows: bool) -> Light {
    Light {
        kind: LightKind::Point,
        x: fixed(x),
        y: fixed(y),
        reach: fixed(30.0),
        intensity: fixed(2.0),
        color: None,
        shadows,
        layer: 0,
        instance: None,
        anchor: None,
    }
}

#[test]
fn sdf_shapes_cover_their_area() {
    let kit = kit();
    let cases = [
        Geometry::RoundedRect {
            bounds: bounds(0.0, 0.0, 10.0, 10.0),
            radius: fixed(3.0),
        },
        Geometry::Polygon {
            x: fixed(0.0),
            y: fixed(0.0),
            points: vec![[0.0, 0.0], [10.0, 0.0], [0.0, 10.0]],
        },
        Geometry::Blob {
            x: fixed(5.0),
            y: fixed(5.0),
            circles: vec![[0.0, 0.0, 3.0], [3.0, 0.0, 3.0]],
            smooth: fixed(2.0),
        },
    ];
    let expected = [(80, 100), (40, 65), (25, 60)];
    for (geometry, (low, high)) in cases.into_iter().zip(expected) {
        let cells = render(
            &scene(&kit, (12, 12), 1, vec![shape(geometry.clone(), kit.ink)]),
            0,
        );
        let covered = count(&cells, kit.ink);
        assert!(
            (low..=high).contains(&covered),
            "{geometry:?} covered {covered}"
        );
    }
}

#[test]
fn dome_shading_and_outlines_add_tones() {
    let kit = kit();
    let ball = Geometry::Ellipse(bounds(2.0, 2.0, 20.0, 20.0));
    let flat = render(
        &scene(&kit, (24, 24), 1, vec![shape(ball.clone(), kit.mid)]),
        0,
    );
    assert_eq!(tones(&flat).len(), 2);
    let style = Style {
        shading: Shading::Dome {
            steps: 1,
            soft: false,
        },
        outline: Outline::Full(kit.ink),
        ..Style::default()
    };
    let shaded = render(
        &scene(&kit, (24, 24), 1, vec![styled(ball, kit.mid, style)]),
        0,
    );
    assert!(tones(&shaded).len() >= 4, "{:?}", tones(&shaded));
    assert_eq!(shaded[12 * 24 + 2], kit.ink, "left edge is outlined");
}

#[test]
fn darkness_lights_and_cast_shadows() {
    let kit = kit();
    let wall = Style {
        casts_shadow: true,
        ..Style::default()
    };
    let shapes = vec![
        shape(Geometry::Rect(bounds(0.0, 0.0, 40.0, 9.0)), kit.mid),
        styled(Geometry::Rect(bounds(19.0, 2.0, 2.0, 5.0)), kit.mid, wall),
    ];
    let dark = Environment {
        ambient: Ambient {
            darkness: fixed(1.0),
            ..Ambient::default()
        },
        ..Environment::default()
    };
    let unlit = render(&world(&kit, (40, 9), 1, shapes.clone(), dark.clone()), 0);
    assert_eq!(unlit[4 * 40 + 30], kit.ink, "darkness lowers one tone");
    let lit = Environment {
        lights: vec![lamp(10.0, 4.0, true)],
        ..dark
    };
    let cells = render(&world(&kit, (40, 9), 1, shapes, lit), 0);
    let (near, behind) = (
        cells[4 * 40 + 12].tone.index(),
        cells[4 * 40 + 24].tone.index(),
    );
    assert!(
        near > kit.mid.tone.index(),
        "the lamp brightens nearby pixels"
    );
    assert!(behind < near, "the wall shadows pixels behind it");
}

#[test]
fn colored_lights_switch_to_the_twin_ramp() {
    let kit = kit();
    let floor = vec![shape(Geometry::Rect(bounds(0.0, 0.0, 16.0, 4.0)), kit.mid)];
    let warm = Light {
        color: Some(kit.warm),
        ..lamp(8.0, 2.0, false)
    };
    let environment = Environment {
        lights: vec![warm],
        ..Environment::default()
    };
    let cells = render(&world(&kit, (16, 4), 1, floor, environment), 0);
    assert_ne!(cells[2 * 16 + 8].ramp, kit.mid.ramp);
}

#[test]
fn clips_layers_and_parallax() {
    let kit = kit();
    let clipped = Style {
        clips: vec![0],
        ..Style::default()
    };
    let far = Style {
        layer: 1,
        ..Style::default()
    };
    let shapes = vec![
        styled(Geometry::Rect(bounds(0.0, 0.0, 8.0, 1.0)), kit.ink, clipped),
        styled(Geometry::Rect(bounds(4.0, 1.0, 1.0, 1.0)), kit.ink, far),
    ];
    let environment = Environment {
        layers: vec![
            Layer::default(),
            Layer {
                parallax: 0.5,
                ..Layer::default()
            },
        ],
        clips: vec![Clip {
            bounds: bounds(2.0, 0.0, 3.0, 1.0),
            round: false,
            layer: 0,
            instance: None,
        }],
        camera: px_engine::Camera {
            x: fixed(4.0),
            y: fixed(0.0),
            zoom: fixed(1.0),
        },
        ..Environment::default()
    };
    let cells = render(&world(&kit, (8, 2), 1, shapes, environment), 0);
    assert_eq!(
        count(&cells[..8], kit.ink),
        1,
        "the clip scrolls with its layer: columns 2..5 land on -2..1"
    );
    assert_eq!(
        cells[8 + 2],
        kit.ink,
        "half-speed layer moves 2px for a 4px camera move"
    );
}

#[test]
fn visibility_and_reflections() {
    let kit = kit();
    let hidden = Style {
        visible: expr("0"),
        ..Style::default()
    };
    let shapes = vec![
        shape(Geometry::Rect(bounds(1.0, 0.0, 1.0, 2.0)), kit.ink),
        styled(Geometry::Rect(bounds(5.0, 0.0, 1.0, 2.0)), kit.ink, hidden),
    ];
    let environment = Environment {
        reflections: vec![Reflection {
            bounds: bounds(0.0, 4.0, 8.0, 4.0),
            axis: fixed(4.0),
            ripple: fixed(0.0),
            speed: fixed(0.0),
            tone_shift: fixed(0.0),
            opacity: fixed(1.0),
            surface: None,
            instance: None,
        }],
        ..Environment::default()
    };
    let cells = render(&world(&kit, (8, 8), 1, shapes, environment), 0);
    assert_eq!(
        count(&cells, kit.ink),
        4,
        "two drawn pixels and their two mirrored copies"
    );
    assert_eq!(cells[6 * 8 + 1], kit.ink);
}
