#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use px_core::{CanvasMut, Paint, Palette, Rgb, Size, Tone};
use px_raster::{Brush, Point, Rect, draw_line, fill_capsule, fill_ellipse, fill_rect};

struct Fixture {
    cells: Vec<Paint>,
    size: Size,
    ink: Paint,
}

impl Fixture {
    fn new(width: u16, height: u16) -> Self {
        let mut palette = Palette::new();
        let ramp = palette
            .push_ramp(&[Rgb::new(0, 0, 0), Rgb::new(1, 1, 1)])
            .unwrap();
        let blank = Paint {
            ramp,
            tone: Tone::new(0),
        };
        let size = Size::new(width, height).unwrap();
        Self {
            cells: vec![blank; size.cell_count()],
            size,
            ink: Paint {
                ramp,
                tone: Tone::new(1),
            },
        }
    }

    fn draw(mut self, draw: impl FnOnce(&mut Brush<'_, '_>)) -> String {
        let ink = self.ink;
        let mut canvas = CanvasMut::new(self.size, &mut self.cells).unwrap();
        draw(&mut Brush::new(&mut canvas, ink));
        self.cells
            .chunks(usize::from(self.size.width()))
            .map(|row| {
                row.iter()
                    .map(|cell| if *cell == ink { '#' } else { '.' })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[test]
fn rect_is_clipped_to_the_canvas() {
    let grid = Fixture::new(4, 3).draw(|c| {
        fill_rect(
            c,
            Rect {
                x: -1,
                y: 1,
                width: 3,
                height: 9,
            },
        );
    });
    assert_eq!(grid, "....\n##..\n##..");
}

#[test]
fn empty_rects_draw_nothing() {
    let grid = Fixture::new(2, 2).draw(|c| {
        fill_rect(
            c,
            Rect {
                x: 0,
                y: 0,
                width: 0,
                height: -2,
            },
        );
    });
    assert_eq!(grid, "..\n..");
}

#[test]
fn ellipse_is_symmetric_for_odd_sizes() {
    let grid = Fixture::new(5, 5).draw(|c| {
        fill_ellipse(
            c,
            Rect {
                x: 0,
                y: 0,
                width: 5,
                height: 5,
            },
        );
    });
    assert_eq!(grid, ".###.\n#####\n#####\n#####\n.###.");
}

#[test]
fn ellipse_is_symmetric_for_even_sizes() {
    let grid = Fixture::new(4, 4).draw(|c| {
        fill_ellipse(
            c,
            Rect {
                x: 0,
                y: 0,
                width: 4,
                height: 4,
            },
        );
    });
    assert_eq!(grid, ".##.\n####\n####\n.##.");
}

#[test]
fn one_pixel_ellipse_is_one_pixel() {
    let grid = Fixture::new(3, 3).draw(|c| {
        fill_ellipse(
            c,
            Rect {
                x: 1,
                y: 1,
                width: 1,
                height: 1,
            },
        );
    });
    assert_eq!(grid, "...\n.#.\n...");
}

#[test]
fn huge_ellipses_are_clamped_not_overflowing() {
    let grid = Fixture::new(2, 2).draw(|c| {
        fill_ellipse(
            c,
            Rect {
                x: i32::MIN,
                y: i32::MIN,
                width: i32::MAX,
                height: i32::MAX,
            },
        );
    });
    assert_eq!(grid.len(), 5);
}

#[test]
fn diagonal_line_has_no_doubled_corners() {
    let grid =
        Fixture::new(5, 3).draw(|c| draw_line(c, Point { x: 0, y: 0 }, Point { x: 4, y: 2 }));
    assert_eq!(grid, "#....\n.##..\n...##");
}

#[test]
fn lines_never_form_l_corners_at_any_slope() {
    for (x, y) in [
        (7, 1),
        (7, 3),
        (7, 5),
        (7, 7),
        (5, 7),
        (1, 7),
        (-7, 3),
        (3, -7),
    ] {
        let grid = Fixture::new(15, 15)
            .draw(|c| draw_line(c, Point { x: 7, y: 7 }, Point { x: 7 + x, y: 7 + y }));
        let rows: Vec<&[u8]> = grid.lines().map(str::as_bytes).collect();
        for pair in rows.windows(2) {
            for col in 0..14 {
                let lit: u8 = [
                    pair[0][col],
                    pair[0][col + 1],
                    pair[1][col],
                    pair[1][col + 1],
                ]
                .iter()
                .map(|cell| u8::from(*cell == b'#'))
                .sum();
                assert!(lit < 3, "L-corner for slope ({x},{y}):\n{grid}");
            }
        }
    }
}

#[test]
fn line_endpoints_are_inclusive_in_both_directions() {
    let forward =
        Fixture::new(3, 1).draw(|c| draw_line(c, Point { x: 0, y: 0 }, Point { x: 2, y: 0 }));
    let backward =
        Fixture::new(3, 1).draw(|c| draw_line(c, Point { x: 2, y: 0 }, Point { x: 0, y: 0 }));
    assert_eq!((forward.as_str(), backward.as_str()), ("###", "###"));
}

#[test]
fn far_off_canvas_lines_terminate() {
    let grid = Fixture::new(2, 2).draw(|c| {
        draw_line(c, Point { x: i32::MIN, y: 0 }, Point { x: i32::MAX, y: 1 });
    });
    assert_eq!(grid.len(), 5);
}

#[test]
fn float_noise_does_not_move_sdf_edges() {
    let exact = Fixture::new(7, 4).draw(|c| fill_capsule(c, [1.0, 1.5], [5.0, 1.5], 1.0));
    let noisy = Fixture::new(7, 4).draw(|c| fill_capsule(c, [1.0, 1.5 - 1e-15], [5.0, 1.5], 1.0));
    assert_eq!(exact, noisy);
    assert_eq!(exact.lines().filter(|row| row.contains('#')).count(), 3);
}
