#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use common::{bounds, fixed, kit, render, scene, shape, world};
use px_core::{Paint, hash_cells};
use px_engine::{Detail, Environment, Geometry, Light, LightKind, ModelError, Shape, Style};

fn covered(cells: &[Paint], paint: Paint) -> Vec<usize> {
    cells
        .iter()
        .enumerate()
        .filter(|(_, cell)| **cell == paint)
        .map(|(index, _)| index)
        .collect()
}

#[test]
fn detail_redraws_shapes_with_more_pixels_in_the_same_place() {
    let kit = kit();
    let square = vec![shape(Geometry::Rect(bounds(2.0, 2.0, 4.0, 4.0)), kit.ink)];
    let doubled = scene(&kit, (8, 8), 1, square)
        .with_detail(Detail::whole(2))
        .unwrap();
    assert_eq!((doubled.size().width(), doubled.size().height()), (16, 16));
    assert_eq!(
        (doubled.spec().size.width(), doubled.spec().size.height()),
        (8, 8)
    );
    let inked = covered(&render(&doubled, 0), kit.ink);
    assert_eq!(inked.len(), 64, "a 4×4 square becomes 8×8");
    assert_eq!(inked.first(), Some(&(4 * 16 + 4)));
}

#[test]
fn native_detail_renders_exactly_as_before() {
    let kit = kit();
    let ball = vec![shape(
        Geometry::Ellipse(bounds(1.5, 2.0, 9.0, 7.0)),
        kit.ink,
    )];
    let plain = scene(&kit, (12, 12), 1, ball);
    let native = plain.clone().with_detail(Detail::NATIVE).unwrap();
    assert_eq!(
        hash_cells(&render(&plain, 0)),
        hash_cells(&render(&native, 0))
    );
}

#[test]
fn detail_takes_numbers_from_one_to_eight() {
    assert_eq!(Detail::new(1.0), Ok(Detail::NATIVE));
    assert_eq!(Detail::new(2.0), Ok(Detail::whole(2)));
    assert!((Detail::new(1.875).unwrap().get() - 1.875).abs() < f64::EPSILON);
    for refused in [0.5, 8.5, f64::NAN, f64::INFINITY] {
        assert!(Detail::new(refused).is_err(), "{refused} was accepted");
    }
    assert_eq!(Detail::whole(0), Detail::NATIVE);
    assert_eq!(Detail::whole(200), Detail::whole(8));
    assert_eq!(Detail::new(2.5).unwrap().grain(), 2);
}

#[test]
fn a_canvas_past_the_size_limit_is_refused() {
    let kit = kit();
    let large = scene(&kit, (1024, 4), 1, Vec::new());
    assert!(matches!(
        large.with_detail(Detail::whole(4)),
        Err(ModelError::TooLarge { .. })
    ));
}

#[test]
fn shadows_fall_behind_casters_at_every_detail() {
    let kit = kit();
    let floor = shape(Geometry::Rect(bounds(0.0, 0.0, 32.0, 8.0)), kit.mid);
    let wall = Shape {
        style: Style {
            casts_shadow: true,
            ..Style::default()
        },
        ..shape(Geometry::Rect(bounds(12.0, 0.0, 2.0, 8.0)), kit.ink)
    };
    let lamp = Light {
        kind: LightKind::Point,
        x: fixed(4.0),
        y: fixed(4.0),
        reach: fixed(30.0),
        intensity: fixed(2.0),
        color: None,
        shadows: true,
        layer: 0,
        instance: None,
        anchor: None,
    };
    let environment = Environment {
        lights: vec![lamp],
        ..Environment::default()
    };
    let native = world(&kit, (32, 8), 1, vec![floor, wall], environment);
    for factor in [1, 2, 3] {
        let detailed = native.clone().with_detail(Detail::whole(factor)).unwrap();
        let width = usize::from(detailed.size().width());
        let cells = render(&detailed, 0);
        let at = |x: usize| cells[4 * usize::from(factor) * width + x * usize::from(factor)];
        assert_ne!(at(8), kit.mid, "detail {factor}: the lamp side is lit");
        assert_eq!(
            at(20),
            kit.mid,
            "detail {factor}: the far side is in shadow"
        );
    }
}
