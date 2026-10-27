//! Frame rendering behavior.
#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use common::{expr, kit, rect, scene};
use px_core::{CanvasMut, Frame, Size, hash_cells};
use px_engine::{FrameBuffer, MAX_SHAPES, ModelError, Param, RenderError, Scene, SceneSpec};
use px_expr::EvalError;

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
    // One full sweep across 4 pixels per second at 12 fps; the scene lasts 1 s.
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
fn evaluation_errors_name_the_shape() {
    let kit = kit();
    let shapes = vec![
        rect(Param::Fixed(0.0), Param::Fixed(0.0), 1.0, 1.0, kit.ink),
        rect(expr("1 / (t - t)"), Param::Fixed(0.0), 1.0, 1.0, kit.ink),
    ];
    let scene = scene(&kit, (2, 2), 1, shapes);
    let error = FrameBuffer::new(&scene)
        .render(&scene, Frame::new(0))
        .unwrap_err();
    assert_eq!(
        error,
        RenderError::Eval {
            shape: 1,
            error: EvalError::NonFinite
        }
    );
}

#[test]
fn canvas_size_must_match_the_scene() {
    let kit = kit();
    let scene = scene(&kit, (2, 2), 1, Vec::new());
    let mut cells = vec![kit.blank; 9];
    let mut canvas = CanvasMut::new(Size::new(3, 3).unwrap(), &mut cells).unwrap();
    let error = px_engine::render_frame(&scene, Frame::new(0), &mut canvas).unwrap_err();
    assert!(matches!(error, RenderError::CanvasSize { .. }));
}

#[test]
fn scene_validation_rejects_bad_paints_and_too_many_shapes() {
    let kit = kit();
    let base = scene(&kit, (1, 1), 1, Vec::new());
    let spec = |shapes| SceneSpec {
        name: "x".to_owned(),
        size: base.size(),
        timing: base.timing(),
        palette: kit.palette.clone(),
        background: kit.blank,
        shapes,
    };
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
        Err(ModelError::TooManyShapes { .. })
    ));
}
