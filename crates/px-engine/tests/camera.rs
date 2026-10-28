#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use common::{bounds, fixed, kit, render, shape, world};
use px_core::Paint;
use px_engine::{Camera, Environment, Geometry};

fn covered(cells: &[Paint], paint: Paint) -> Vec<usize> {
    cells
        .iter()
        .enumerate()
        .filter(|(_, cell)| **cell == paint)
        .map(|(index, _)| index)
        .collect()
}

fn with_camera(x: f64, zoom: f64) -> Environment {
    Environment {
        camera: Camera {
            x: fixed(x),
            y: fixed(0.0),
            zoom: fixed(zoom),
        },
        ..Environment::default()
    }
}

#[test]
fn camera_pans_scenes_without_layers() {
    let kit = kit();
    let square = vec![shape(Geometry::Rect(bounds(4.0, 0.0, 2.0, 1.0)), kit.ink)];
    let cells = render(&world(&kit, (8, 1), 1, square, with_camera(3.0, 1.0)), 0);
    assert_eq!(covered(&cells, kit.ink), vec![1, 2]);
}

#[test]
fn zoom_redraws_shapes_larger_about_the_center() {
    let kit = kit();
    let square = vec![shape(Geometry::Rect(bounds(6.0, 6.0, 4.0, 4.0)), kit.ink)];
    let cells = render(&world(&kit, (16, 16), 1, square, with_camera(0.0, 2.0)), 0);
    let inked = covered(&cells, kit.ink);
    assert_eq!(inked.len(), 64, "a 4×4 square becomes 8×8");
    assert_eq!(inked.first(), Some(&(4 * 16 + 4)));
}

#[test]
fn zoom_keeps_outlines_one_pixel_wide() {
    let kit = kit();
    let ball = Geometry::Ellipse(bounds(4.0, 4.0, 8.0, 8.0));
    let mut outlined = shape(ball, kit.ink);
    outlined.style.outline = px_engine::Outline::Full(kit.mid);
    let cells = render(
        &world(&kit, (16, 16), 1, vec![outlined], with_camera(0.0, 1.5)),
        0,
    );
    let row = &cells[8 * 16..9 * 16];
    let first = row.iter().position(|cell| *cell == kit.mid).unwrap();
    assert_eq!(row[first + 1], kit.ink, "outline is a single pixel");
}
