#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use common::{bounds, expr, kit, scene, shape};
use px_core::{Frame, hash_cells};
use px_engine::{FrameBuffer, Geometry, Outline, Shading, Shape};

#[test]
fn a_frame_renders_the_same_whatever_came_before() {
    let kit = kit();
    let mut dome = shape(
        Geometry::Ellipse(px_engine::Bounds {
            x: expr("2 + f * 4"),
            ..bounds(0.0, 2.0, 20.0, 20.0)
        }),
        kit.mid,
    );
    dome.style.shading = Shading::Dome {
        steps: 2,
        soft: false,
    };
    let mut flat = shape(Geometry::Rect(bounds(14.0, 6.0, 12.0, 12.0)), kit.ink);
    flat.style.outline = Outline::Selective(kit.blank);
    let shapes: Vec<Shape> = vec![dome, flat];
    let scene = scene(&kit, (32, 32), 4, shapes);
    let mut fresh = FrameBuffer::new(&scene);
    let alone = hash_cells(fresh.render(&scene, Frame::new(0)).unwrap().cells());
    let mut used = FrameBuffer::new(&scene);
    used.render(&scene, Frame::new(3)).unwrap();
    let after = hash_cells(used.render(&scene, Frame::new(0)).unwrap().cells());
    assert_eq!(alone, after);
}
