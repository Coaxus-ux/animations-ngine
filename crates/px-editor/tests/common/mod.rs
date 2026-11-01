#![allow(
    dead_code,
    clippy::unwrap_used,
    reason = "test helpers; not every test file uses every helper"
)]

use std::path::{Path, PathBuf};

use px_editor::app::{App, WindowId};
use px_editor::assets::ThemeChoice;
pub use px_editor::ports::MemoryFiles;
use px_editor::views::Placement;
use px_ui::{Buttons, Input};

pub const SIZE: (usize, usize) = (1280, 800);
pub const FILE: &str = "room.kdl";

pub fn example(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../examples/{name}"));
    std::fs::read_to_string(path).unwrap()
}

pub fn open() -> (App, MemoryFiles, WindowId) {
    let mut files = MemoryFiles::default();
    files
        .files
        .insert(PathBuf::from(FILE), example("pixel-room.kdl"));
    let mut app = App::new(ThemeChoice::Dark).unwrap();
    app.open(Path::new(FILE), None, &mut files).unwrap();
    let main = app.window_ids().next().unwrap();
    app.frame(main, &Input::default(), SIZE, &mut files);
    (app, files, main)
}

pub fn open_empty() -> (App, MemoryFiles, WindowId) {
    let mut files = MemoryFiles::default();
    let mut app = App::new(ThemeChoice::Dark).unwrap();
    let main = app.window_ids().next().unwrap();
    app.frame(main, &Input::default(), SIZE, &mut files);
    (app, files, main)
}

pub fn layer(app: &App, name: &str) -> usize {
    app.document()
        .unwrap()
        .layers()
        .iter()
        .position(|layer| layer.name == name)
        .unwrap()
}

pub fn window_point(app: &App, window: WindowId, at: (i32, i32)) -> (i32, i32) {
    let state = app.window(window).unwrap();
    let scene = app.document().unwrap().scene().unwrap();
    let size = (
        i32::from(scene.size().width()),
        i32::from(scene.size().height()),
    );
    let placement = Placement::of(&state.view, state.area.unwrap(), size);
    (
        placement.origin.0 + at.0 * placement.zoom + placement.zoom / 2,
        placement.origin.1 + at.1 * placement.zoom + placement.zoom / 2,
    )
}

pub fn stroke(app: &mut App, files: &mut MemoryFiles, window: WindowId, points: &[(i32, i32)]) {
    for (step, point) in points.iter().enumerate() {
        let at = window_point(app, window, *point);
        let first = step == 0;
        let input = Input {
            pointer: Some(at),
            pointer_anywhere: at,
            down: Buttons {
                left: true,
                ..Buttons::default()
            },
            pressed: Buttons {
                left: first,
                ..Buttons::default()
            },
            ..Input::default()
        };
        app.frame(window, &input, SIZE, files);
    }
    let at = window_point(app, window, *points.last().unwrap());
    let release = Input {
        pointer: Some(at),
        pointer_anywhere: at,
        released: Buttons {
            left: true,
            ..Buttons::default()
        },
        ..Input::default()
    };
    app.frame(window, &release, SIZE, files);
}

pub fn click(app: &mut App, files: &mut MemoryFiles, window: WindowId, at: (i32, i32)) {
    let press = Input {
        pointer: Some(at),
        pointer_anywhere: at,
        down: Buttons {
            left: true,
            ..Buttons::default()
        },
        pressed: Buttons {
            left: true,
            ..Buttons::default()
        },
        ..Input::default()
    };
    app.frame(window, &press, SIZE, files);
    let release = Input {
        pointer: Some(at),
        pointer_anywhere: at,
        released: Buttons {
            left: true,
            ..Buttons::default()
        },
        ..Input::default()
    };
    app.frame(window, &release, SIZE, files);
}

pub fn shown_paint(app: &App, at: (i32, i32)) -> px_core::Paint {
    let scene = app.document().unwrap().scene().unwrap();
    let mut buffer = px_engine::FrameBuffer::new(scene);
    let view = buffer.render(scene, px_core::Frame::new(0)).unwrap();
    view.get(at.0, at.1).unwrap()
}

pub fn check_golden(app: &mut App, files: &mut MemoryFiles, window: WindowId, golden: &str) {
    let pixels = app.frame(window, &Input::default(), SIZE, files).unwrap();
    let bytes: Vec<u8> = pixels
        .iter()
        .flat_map(|pixel| pixel.to_le_bytes())
        .collect();
    if let Some(path) = std::env::var_os("PX_EDITOR_SNAPSHOT") {
        let colors: Vec<px_core::Rgb> = pixels
            .iter()
            .map(|pixel| {
                let [_, r, g, b] = pixel.to_be_bytes();
                px_core::Rgb::new(r, g, b)
            })
            .collect();
        let size = px_core::Size::new(1280, 800).unwrap();
        let file = std::fs::File::create(path).unwrap();
        px_export::write_png_colors(file, size, &colors, px_export::Scale::new(1).unwrap())
            .unwrap();
    }
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/golden/{golden}"));
    let actual = format!("{:016x}\n", px_core::hash_bytes(&bytes));
    if std::env::var_os("PX_BLESS").is_some() {
        std::fs::write(&golden, &actual).unwrap();
    }
    assert_eq!(
        actual,
        std::fs::read_to_string(&golden).unwrap_or_default(),
        "rerun with PX_BLESS=1 if the UI change is intended ({})",
        golden.display()
    );
}
