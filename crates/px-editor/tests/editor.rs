//! The editor driven headless: scripted input and an in-memory file store, no window.
#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use px_core::hash_bytes;
use px_editor::app::{App, Intent, WindowId};
use px_editor::assets::ThemeChoice;
use px_editor::ports::FileStore;
use px_editor::tools::ToolKind;
use px_editor::views::Placement;
use px_ui::{Buttons, Input};

const SIZE: (usize, usize) = (1280, 800);
const FILE: &str = "room.kdl";

#[derive(Default)]
struct MemoryFiles {
    files: BTreeMap<PathBuf, String>,
    writes: u64,
}

impl FileStore for MemoryFiles {
    fn read(&mut self, path: &Path) -> Result<String, String> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| "no such file".to_owned())
    }

    fn write(&mut self, path: &Path, text: &str) -> Result<(), String> {
        self.writes += 1;
        self.files.insert(path.to_path_buf(), text.to_owned());
        Ok(())
    }

    fn stamp(&mut self, path: &Path) -> Option<u64> {
        self.files.contains_key(path).then_some(self.writes)
    }
}

fn example() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/pixel-room.kdl");
    std::fs::read_to_string(path).unwrap()
}

/// The app with the pixel-room example open, its main window drawn once.
fn open() -> (App, MemoryFiles, WindowId) {
    let mut files = MemoryFiles::default();
    files.files.insert(PathBuf::from(FILE), example());
    let mut app = App::new(ThemeChoice::Dark).unwrap();
    app.open(Path::new(FILE), None, &mut files).unwrap();
    let main = app.window_ids().next().unwrap();
    app.frame(main, &Input::default(), SIZE, &mut files);
    (app, files, main)
}

fn layer(app: &App, name: &str) -> usize {
    app.document()
        .unwrap()
        .layers()
        .iter()
        .position(|layer| layer.name == name)
        .unwrap()
}

/// Window pixel at the center of canvas pixel `at`.
fn window_point(app: &App, window: WindowId, at: (i32, i32)) -> (i32, i32) {
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

/// Presses at the first canvas pixel, drags through the others, releases at the last.
fn stroke(app: &mut App, files: &mut MemoryFiles, window: WindowId, points: &[(i32, i32)]) {
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

fn shown_paint(app: &App, at: (i32, i32)) -> px_core::Paint {
    let scene = app.document().unwrap().scene().unwrap();
    let mut buffer = px_engine::FrameBuffer::new(scene);
    let view = buffer.render(scene, px_core::Frame::new(0)).unwrap();
    view.get(at.0, at.1).unwrap()
}

#[test]
fn the_ui_draws_the_same_pixels_every_time() {
    let (mut app, mut files, main) = open();
    let pixels = app
        .frame(main, &Input::default(), SIZE, &mut files)
        .unwrap();
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
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/ui.txt");
    let actual = format!("{:016x}\n", hash_bytes(&bytes));
    if std::env::var_os("PX_BLESS").is_some() {
        std::fs::write(&golden, &actual).unwrap();
    }
    assert_eq!(
        actual,
        std::fs::read_to_string(golden).unwrap_or_default(),
        "rerun with PX_BLESS=1 if the UI change is intended"
    );
}

#[test]
fn a_pencil_stroke_is_one_undoable_paint() {
    let (mut app, mut files, main) = open();
    let original = app.document().unwrap().text().to_owned();
    let cat = layer(&app, "cat");
    app.dispatch(Intent::SelectLayer(cat), main, &mut files);
    app.dispatch(Intent::Tool(ToolKind::Pencil), main, &mut files);
    let paint = app.editing().primary.unwrap();
    stroke(&mut app, &mut files, main, &[(2, 2), (3, 2), (4, 2)]);
    assert_ne!(app.document().unwrap().text(), original);
    for x in 2..=4 {
        assert_eq!(shown_paint(&app, (x, 2)), paint);
    }
    app.dispatch(Intent::Undo, main, &mut files);
    assert_eq!(app.document().unwrap().text(), original);
    app.dispatch(Intent::Redo, main, &mut files);
    assert_eq!(shown_paint(&app, (3, 2)), paint);
    app.dispatch(Intent::Save, main, &mut files);
    assert_eq!(files.files[Path::new(FILE)], app.document().unwrap().text());
}

#[test]
fn layers_are_added_renamed_moved_and_deleted_in_the_file() {
    let (mut app, mut files, main) = open();
    let count = app.document().unwrap().layers().len();
    app.dispatch(Intent::AddLayer, main, &mut files);
    let layers = app.document().unwrap().layers();
    assert_eq!(layers.len(), count + 1);
    let added = layers
        .iter()
        .position(|layer| layer.name == "layer-1")
        .unwrap();
    assert!(layers[added].paintable());
    app.dispatch(
        Intent::RenameLayer {
            layer: added,
            name: "ink".to_owned(),
        },
        main,
        &mut files,
    );
    let text = app.document().unwrap().text().to_owned();
    assert!(
        text.contains("layer \"ink\" {") && text.contains("pixels \"ink-art\""),
        "{text}"
    );
    let ink = layer(&app, "ink");
    app.dispatch(Intent::MoveLayer { from: ink, to: 0 }, main, &mut files);
    assert_eq!(app.document().unwrap().layers()[0].name, "ink");
    app.dispatch(Intent::DeleteLayer(0), main, &mut files);
    assert_eq!(app.document().unwrap().layers().len(), count);
    assert_eq!(app.document().unwrap().errors(), [] as [String; 0]);
}

#[test]
fn a_second_window_shows_edits_made_in_the_first() {
    let (mut app, mut files, main) = open();
    app.dispatch(Intent::NewWindow, main, &mut files);
    let second = app.window_ids().last().unwrap();
    let before = app
        .frame(second, &Input::default(), (640, 480), &mut files)
        .unwrap()
        .to_vec();
    let cat = layer(&app, "cat");
    app.dispatch(Intent::SelectLayer(cat), main, &mut files);
    stroke(&mut app, &mut files, main, &[(1, 1), (6, 1)]);
    let after = app
        .frame(second, &Input::default(), (640, 480), &mut files)
        .unwrap()
        .to_vec();
    assert_ne!(before, after);
}

#[test]
fn picking_an_object_and_clicking_a_swatch_recolors_it_in_one_click() {
    let (mut app, mut files, main) = open();
    app.dispatch(Intent::Tool(ToolKind::Pick), main, &mut files);
    stroke(&mut app, &mut files, main, &[(2, 44)]);
    let floor = app.editing().picked.unwrap();
    let scene = app.document().unwrap().scene().unwrap();
    let new = px_core::Paint {
        tone: px_core::Tone::new(3),
        ..scene.shapes()[floor].paint
    };
    let before = app.document().unwrap().text().to_owned();
    app.dispatch(Intent::Swatch(new), main, &mut files);
    let after = app.document().unwrap().text().to_owned();
    let changed: Vec<(&str, &str)> = before
        .lines()
        .zip(after.lines())
        .filter(|(a, b)| a != b)
        .collect();
    assert_eq!(changed.len(), 1, "{changed:?}");
    assert!(changed[0].1.contains("paint=\"wood.3\""), "{changed:?}");
}

#[test]
fn text_typed_in_the_code_view_reloads_and_bad_text_keeps_the_last_scene() {
    let (mut app, mut files, main) = open();
    let text = app
        .document()
        .unwrap()
        .text()
        .replace("width=64 height=48", "width=80 height=48");
    app.dispatch(Intent::SetText(text), main, &mut files);
    assert_eq!(app.document().unwrap().scene().unwrap().size().width(), 80);
    let broken = app
        .document()
        .unwrap()
        .text()
        .replace("paint=\"wall.2\"", "paint=\"nope.2\"");
    app.dispatch(Intent::SetText(broken), main, &mut files);
    assert_ne!(app.document().unwrap().errors().len(), 0);
    assert_eq!(app.document().unwrap().scene().unwrap().size().width(), 80);
}

#[test]
fn torn_off_panels_come_back_with_the_layout() {
    let (mut app, mut files, main) = open();
    app.dispatch(
        Intent::PopOut(px_editor::app::PanelKind::Palette),
        main,
        &mut files,
    );
    let layout = app.layout_text();
    assert!(layout.contains("panel \"Palette\""), "{layout}");
    let mut restored = App::new(ThemeChoice::Dark).unwrap();
    restored.apply_layout(&layout);
    assert_eq!(restored.window_ids().count(), 2);
    let requests = restored.take_requests();
    assert!(requests.iter().any(|request| matches!(request, px_editor::app::Request::Open { title, .. } if title.starts_with("Palette"))));
}
