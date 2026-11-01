#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use std::path::{Path, PathBuf};

use common::{MemoryFiles, SIZE, check_golden, click};
use px_editor::app::{App, WindowId};
use px_editor::assets::ThemeChoice;
use px_editor::domain::Document;
use px_editor::domain::timeline::{KeyRow, MarkerAt, key_rows, markers};
use px_ui::{Buttons, HEADER, Input, ROW};

const FILE: &str = "keys.kdl";
const KEYED: &str = r##"palette "p" {
    ramp "ink" "#000000" "#808080" "#ffffff"
}

scene "keys" palette="p" width=32 height=24 fps=12 duration="2s" background="ink.0" {
    timeline {
        marker "start" at="0s"
        marker "half" at="1s"
    }
    camera {
        k "0s" x=0
        k "1.5s" x=4
    }
    rect "box" x=0 y=0 width=4 height=4 paint="ink.2" {
        anim x {
            k "0s" 0
            k "half" 10
            k "20f" 20
        }
    }
    rect x=8 y=8 width=2 height=2 paint="ink.1"
}
"##;

const NAME: i32 = 44;
const CELL: i32 = 8;
const BAR: i32 = ROW + 3;
const ROW_HEIGHT: i32 = CELL + 2;

fn open_keyed() -> (App, MemoryFiles, WindowId) {
    let mut files = MemoryFiles::default();
    files.files.insert(PathBuf::from(FILE), KEYED.to_owned());
    let mut app = App::new(ThemeChoice::Dark).unwrap();
    app.open(Path::new(FILE), None, &mut files).unwrap();
    let main = app.window_ids().next().unwrap();
    app.frame(main, &Input::default(), SIZE, &mut files);
    (app, files, main)
}

fn frame_of(app: &App, main: WindowId) -> u32 {
    app.window(main).unwrap().view.frame
}

fn grid_point(app: &App, main: WindowId, frame: u32, row: i32) -> (i32, i32) {
    let scale = app.scale_for(SIZE.0);
    let canvas = app.window(main).unwrap().area.unwrap();
    let body = (
        canvas.x + 2 * scale,
        canvas.y + canvas.height + (HEADER + 2) * scale,
    );
    let column = NAME + i32::try_from(frame).unwrap() * CELL + CELL / 2;
    let y = BAR + row * ROW_HEIGHT + ROW_HEIGHT / 2;
    (body.0 + column * scale, body.1 + y * scale)
}

#[test]
fn markers_and_keys_are_reported_in_frames_per_animated_object() {
    let mut files = MemoryFiles::default();
    let document = Document::from_text(Path::new(FILE), KEYED.to_owned(), &mut files).unwrap();
    assert_eq!(
        markers(&document),
        vec![
            MarkerAt {
                name: "start".to_owned(),
                frame: 0
            },
            MarkerAt {
                name: "half".to_owned(),
                frame: 12
            },
        ]
    );
    let rows = key_rows(&document);
    let summary: Vec<(&str, &[u32])> = rows
        .iter()
        .map(|row: &KeyRow| (row.label.as_str(), row.frames.as_slice()))
        .collect();
    assert_eq!(
        summary,
        vec![("camera", &[0, 18][..]), ("box", &[0, 12, 20][..])]
    );
    let path = rows[1].path.as_ref().unwrap();
    let node = document.source().node(path).unwrap();
    assert_eq!(node.name().value(), "rect");
}

#[test]
fn a_scene_without_keys_or_markers_has_no_key_rows() {
    let (app, _files, _main) = common::open();
    let document = app.document().unwrap();
    assert_eq!(markers(document), Vec::new());
    assert_eq!(key_rows(document), Vec::new());
}

#[test]
fn clicking_the_ruler_jumps_to_that_frame() {
    let (mut app, mut files, main) = open_keyed();
    let at = grid_point(&app, main, 7, 0);
    click(&mut app, &mut files, main, at);
    assert_eq!(frame_of(&app, main), 7);
}

#[test]
fn dragging_along_the_ruler_scrubs_through_the_frames() {
    let (mut app, mut files, main) = open_keyed();
    let held = Buttons {
        left: true,
        ..Buttons::default()
    };
    for (step, frame) in [3, 9, 15].into_iter().enumerate() {
        let at = grid_point(&app, main, frame, 0);
        let input = Input {
            pointer: Some(at),
            pointer_anywhere: at,
            down: held,
            pressed: Buttons {
                left: step == 0,
                ..Buttons::default()
            },
            ..Input::default()
        };
        app.frame(main, &input, SIZE, &mut files);
        app.frame(
            main,
            &Input {
                pressed: Buttons::default(),
                ..input
            },
            SIZE,
            &mut files,
        );
        assert_eq!(frame_of(&app, main), frame);
    }
}

#[test]
fn clicking_a_marker_jumps_to_it() {
    let (mut app, mut files, main) = open_keyed();
    let at = grid_point(&app, main, 14, 1);
    click(&mut app, &mut files, main, at);
    assert_eq!(frame_of(&app, main), 12);
}

#[test]
fn clicking_a_key_selects_its_element_and_jumps_to_its_frame() {
    let (mut app, mut files, main) = open_keyed();
    let at = grid_point(&app, main, 20, 3);
    click(&mut app, &mut files, main, at);
    assert_eq!(frame_of(&app, main), 20);
    let selected = app.window(main).unwrap().selected.clone().unwrap();
    let document = app.document().unwrap();
    assert_eq!(key_rows(document)[1].path.as_ref(), Some(&selected));
}

#[test]
fn the_timeline_of_a_keyed_scene_matches_its_golden_image() {
    let (mut app, mut files, main) = open_keyed();
    check_golden(&mut app, &mut files, main, "ui-timeline-keys.txt");
}
