#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use common::{expr, kit, rect, scene, spec};
use px_core::{Frame, hash_cells};
use px_engine::{
    Environment, FrameBuffer, FrameReport, MAX_SHAPES, ModelError, Param, RenderError, Scene,
};

#[test]
fn shapes_draw_in_order_over_the_background() {
    let kit = kit();
    let shapes = vec![
        rect(Param::Fixed(0.0), Param::Fixed(0.0), 2.0, 1.0, kit.ink),
        rect(Param::Fixed(1.0), Param::Fixed(0.0), 1.0, 1.0, kit.blank),
    ];
    let scene = scene(&kit, (3, 1), 1, shapes);
    let mut buffer = FrameBuffer::new(&scene);
    let view = buffer.render(&scene, Frame::new(0)).unwrap();
    assert_eq!(view.cells(), [kit.ink, kit.blank, kit.blank]);
}

#[test]
fn expressions_animate_and_periodic_scenes_loop() {
    let kit = kit();
    let shapes = vec![rect(
        expr("floor(t * 4) % 4"),
        Param::Fixed(0.0),
        1.0,
        1.0,
        kit.ink,
    )];
    let scene = scene(&kit, (4, 1), 12, shapes);
    let mut buffer = FrameBuffer::new(&scene);
    let first = hash_cells(buffer.render(&scene, Frame::new(0)).unwrap().cells());
    let moved = hash_cells(buffer.render(&scene, Frame::new(3)).unwrap().cells());
    let wrapped = hash_cells(buffer.render(&scene, Frame::new(12)).unwrap().cells());
    assert_ne!(first, moved);
    assert_eq!(first, wrapped);
}

#[test]
fn non_finite_values_become_zero_and_are_reported() {
    let kit = kit();
    let shapes = vec![
        rect(Param::Fixed(0.0), Param::Fixed(0.0), 1.0, 1.0, kit.blank),
        rect(expr("1 / (t - t)"), Param::Fixed(0.0), 1.0, 1.0, kit.ink),
    ];
    let scene = scene(&kit, (2, 1), 1, shapes);
    let mut buffer = FrameBuffer::new(&scene);
    let view = buffer.render(&scene, Frame::new(0)).unwrap();
    assert_eq!(view.cells(), [kit.ink, kit.blank]);
    assert_eq!(
        buffer.report(),
        FrameReport {
            non_finite_values: 1,
            first_non_finite_shape: Some(1)
        }
    );
}

#[test]
fn buffer_size_must_match_the_scene() {
    let kit = kit();
    let small = scene(&kit, (2, 2), 1, Vec::new());
    let large = scene(&kit, (3, 3), 1, Vec::new());
    let mut buffer = FrameBuffer::new(&large);
    let error = buffer.render(&small, Frame::new(0)).unwrap_err();
    assert!(matches!(error, RenderError::CanvasSize { .. }));
}

#[test]
fn scene_validation_rejects_bad_paints_and_too_many_shapes() {
    let kit = kit();
    let spec = |shapes| spec(&kit, (1, 1), 1, shapes, Environment::default());
    let foreign = px_core::Paint {
        ramp: kit.ink.ramp,
        tone: px_core::Tone::new(7),
    };
    let bad = Scene::new(spec(vec![rect(
        Param::Fixed(0.0),
        Param::Fixed(0.0),
        1.0,
        1.0,
        foreign,
    )]));
    assert!(matches!(
        bad,
        Err(ModelError::InvalidShapePaint { shape: 0, .. })
    ));
    let many = vec![rect(Param::Fixed(0.0), Param::Fixed(0.0), 1.0, 1.0, kit.ink); MAX_SHAPES + 1];
    assert!(matches!(
        Scene::new(spec(many)),
        Err(ModelError::TooMany { what: "shapes", .. })
    ));
}
