#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use px_core::{
    COORD_LIMIT, CanvasError, CanvasMut, CanvasView, Frame, FrameCount, FrameRate, MAX_CANVAS_SIDE,
    Paint, Palette, Rgb, Size, Tone, hash_cells, snap_coord, snap_extent,
};

fn paints() -> (Paint, Paint) {
    let mut palette = Palette::new();
    let ramp = palette
        .push_ramp(&[Rgb::new(0, 0, 0), Rgb::new(9, 9, 9)])
        .unwrap();
    (
        Paint {
            ramp,
            tone: Tone::new(0),
        },
        Paint {
            ramp,
            tone: Tone::new(1),
        },
    )
}

#[test]
fn size_rejects_zero_and_oversized_sides() {
    assert!(Size::new(0, 4).is_err());
    assert!(Size::new(4, MAX_CANVAS_SIDE + 1).is_err());
    assert_eq!(Size::new(3, 2).map(Size::cell_count), Ok(6));
}

#[test]
fn canvas_requires_an_exact_buffer_length() {
    let (dark, _) = paints();
    let size = Size::new(2, 2).unwrap();
    let mut cells = vec![dark; 3];
    let error = CanvasMut::new(size, &mut cells).unwrap_err();
    assert_eq!(
        error,
        CanvasError::BufferLength {
            expected: 4,
            actual: 3
        }
    );
}

#[test]
fn set_and_fill_span_clip_to_the_canvas() {
    let (dark, light) = paints();
    let size = Size::new(4, 2).unwrap();
    let mut cells = vec![dark; size.cell_count()];
    let mut canvas = CanvasMut::new(size, &mut cells).unwrap();
    canvas.set(-1, 0, light);
    canvas.set(4, 1, light);
    canvas.fill_span(1, -5, 2, light);
    canvas.fill_span(0, 3, 99, light);
    canvas.fill_span(5, 0, 4, light);
    let view = canvas.view();
    let lit: Vec<(i32, i32)> = (0..2)
        .flat_map(|y| (0..4).map(move |x| (x, y)))
        .filter(|&(x, y)| view.get(x, y) == Some(light))
        .collect();
    assert_eq!(lit, [(3, 0), (0, 1), (1, 1)]);
    assert_eq!(view.get(4, 0), None);
}

#[test]
fn snapping_rounds_half_away_from_zero_and_clamps() {
    assert_eq!(snap_coord(2.4), 2);
    assert_eq!(snap_coord(2.5), 3);
    assert_eq!(snap_coord(-2.5), -3);
    assert_eq!(snap_coord(-0.4), 0);
    assert_eq!(
        snap_coord(2.5 - 1e-12),
        3,
        "float noise just below a half must still round up"
    );
    assert_eq!(snap_coord(36.0 - 2.4e-16), 36);
    assert_eq!(snap_coord(1e12), COORD_LIMIT);
    assert_eq!(snap_coord(-1e12), -COORD_LIMIT);
    assert_eq!(snap_coord(f64::NAN), 0);
    assert_eq!(snap_extent(-3.0), 0);
}

#[test]
fn hash_is_stable_and_order_sensitive() {
    let (dark, light) = paints();
    let size = Size::new(2, 1).unwrap();
    let a = [dark, light];
    let b = [light, dark];
    assert_eq!(
        CanvasView::new(size, &a).map(|v| hash_cells(v.cells())),
        Ok(hash_cells(&a))
    );
    assert_ne!(hash_cells(&a), hash_cells(&b));
    // Pinned value: changing the hash breaks every golden file, so it must be deliberate.
    assert_eq!(hash_cells(&a), 0xd7e4_fbfa_299d_6f8a);
}

#[test]
fn time_types_validate_ranges() {
    assert!(FrameRate::new(0).is_err());
    let fps = FrameRate::new(12).unwrap();
    assert!((fps.seconds_at(Frame::new(6)) - 0.5).abs() < f64::EPSILON);
    assert!(FrameCount::new(0).is_err());
    assert_eq!(
        FrameCount::new(3)
            .unwrap()
            .frames()
            .map(Frame::index)
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
}
