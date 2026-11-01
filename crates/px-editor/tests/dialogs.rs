//! The Canvas Size and Properties dialogs and what they do to the document, driven headless.
#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use common::{SIZE, layer, open, shown_paint, stroke};
use px_editor::app::{App, DialogKind, Intent};
use px_editor::domain::{Anchor, ElementKind, tree};
use px_editor::tools::ToolKind;
use px_engine::Looping;
use px_ui::{Input, Key};

fn size(app: &App) -> (u16, u16) {
    let size = app.document().unwrap().scene().unwrap().size();
    (size.width(), size.height())
}

/// Paints one pixel of the cat layer at `at` and returns its paint.
fn paint_pixel(
    app: &mut App,
    files: &mut common::MemoryFiles,
    main: px_editor::app::WindowId,
    at: (i32, i32),
) -> px_core::Paint {
    let cat = layer(app, "cat");
    app.dispatch(Intent::SelectLayer(cat), main, files);
    app.dispatch(Intent::Tool(ToolKind::Pencil), main, files);
    let paint = app.editing().primary.unwrap();
    stroke(app, files, main, &[at]);
    assert_eq!(shown_paint(app, at), paint);
    paint
}

#[test]
fn canvas_size_with_a_center_anchor_shifts_cels_by_half_the_delta_in_one_undo_step() {
    let (mut app, mut files, main) = open();
    let paint = paint_pixel(&mut app, &mut files, main, (2, 2));
    let before = app.document().unwrap().text().to_owned();
    app.dispatch(
        Intent::ResizeCanvas {
            width: 80,
            height: 60,
            anchor: Anchor::Center,
        },
        main,
        &mut files,
    );
    assert_eq!(size(&app), (80, 60));
    assert_eq!(app.document().unwrap().errors(), [] as [String; 0]);
    assert_eq!(shown_paint(&app, (10, 8)), paint);
    assert_ne!(shown_paint(&app, (2, 2)), paint);
    assert!(
        app.editing().status.contains("80×60"),
        "{}",
        app.editing().status
    );

    app.dispatch(Intent::Undo, main, &mut files);
    assert_eq!(size(&app), (64, 48));
    assert_eq!(app.document().unwrap().text(), before);
    assert_eq!(shown_paint(&app, (2, 2)), paint);
}

#[test]
fn shrinking_crops_pixels_outside_the_canvas() {
    let (mut app, mut files, main) = open();
    let paint = paint_pixel(&mut app, &mut files, main, (33, 5));
    let kept = paint_pixel(&mut app, &mut files, main, (3, 3));
    let resize = |width, height| Intent::ResizeCanvas {
        width,
        height,
        anchor: Anchor::TopLeft,
    };
    app.dispatch(resize(32, 48), main, &mut files);
    assert_eq!(size(&app), (32, 48));
    assert_eq!(shown_paint(&app, (3, 3)), kept);
    app.dispatch(resize(64, 48), main, &mut files);
    assert_eq!(size(&app), (64, 48));
    assert_ne!(
        shown_paint(&app, (60, 2)),
        paint,
        "the cropped pixel is gone"
    );
    assert_eq!(shown_paint(&app, (3, 3)), kept);
}

#[test]
fn properties_rewrites_fps_duration_and_loop() {
    let (mut app, mut files, main) = open();
    app.dispatch(
        Intent::SetTiming {
            fps: 24,
            duration: 30,
            looping: false,
        },
        main,
        &mut files,
    );
    let timing = app.document().unwrap().scene().unwrap().timing();
    assert_eq!(timing.fps.get(), 24);
    assert_eq!(timing.duration.get(), 30);
    assert_eq!(timing.looping, Looping::Once);
    let text = app.document().unwrap().text();
    assert!(
        text.contains("fps=24 duration=\"30f\" loop=#false"),
        "{text}"
    );
    assert!(
        app.editing().status.contains("3 undo steps"),
        "{}",
        app.editing().status
    );

    app.dispatch(
        Intent::SetTiming {
            fps: 24,
            duration: 30,
            looping: false,
        },
        main,
        &mut files,
    );
    assert!(
        app.editing().status.contains("already"),
        "{}",
        app.editing().status
    );
}

#[test]
fn shortcuts_are_ignored_while_a_dialog_is_open() {
    let (mut app, mut files, main) = open();
    let cat = layer(&app, "cat");
    app.dispatch(Intent::SelectLayer(cat), main, &mut files);
    let before = app.document().unwrap().text().to_owned();
    let flip = Input {
        keys: vec![Key::Letter('h')],
        ..Input::default()
    };
    app.dispatch(Intent::ShowDialog(DialogKind::CanvasSize), main, &mut files);
    app.frame(main, &flip, SIZE, &mut files);
    assert_eq!(app.document().unwrap().text(), before);
    assert!(app.window(main).unwrap().dialog.is_some());

    app.dispatch(Intent::CloseDialog, main, &mut files);
    app.frame(main, &flip, SIZE, &mut files);
    assert_ne!(
        app.document().unwrap().text(),
        before,
        "H flips once the dialog is gone"
    );
}

#[test]
fn escape_closes_the_dialog_without_changes() {
    let (mut app, mut files, main) = open();
    let before = app.document().unwrap().text().to_owned();
    app.dispatch(Intent::ShowDialog(DialogKind::CanvasSize), main, &mut files);
    app.frame(main, &Input::default(), SIZE, &mut files);
    assert!(app.window(main).unwrap().dialog.is_some());
    let escape = Input {
        keys: vec![Key::Escape],
        ..Input::default()
    };
    app.frame(main, &escape, SIZE, &mut files);
    assert!(app.window(main).unwrap().dialog.is_none());
    assert_eq!(app.document().unwrap().text(), before);
    assert_eq!(size(&app), (64, 48));
}

#[test]
fn the_canvas_size_dialog_opens_with_the_current_size() {
    let (mut app, mut files, main) = open();
    app.dispatch(Intent::ShowDialog(DialogKind::CanvasSize), main, &mut files);
    let form = &app.window(main).unwrap().dialog.as_ref().unwrap().form;
    assert_eq!(form.numbers["width"], 64);
    assert_eq!(form.numbers["height"], 48);
    assert_eq!(form.choices["anchor"], 4);

    app.dispatch(Intent::ShowDialog(DialogKind::Properties), main, &mut files);
    let form = &app.window(main).unwrap().dialog.as_ref().unwrap().form;
    assert_eq!(form.texts["name"], "pixel-room");
    assert_eq!(form.texts["background"], "wall.0");
    assert_eq!(form.numbers["fps"], 12);
    assert_eq!(form.numbers["duration"], 18);
    assert!(form.flags["loop"]);
}

#[test]
fn a_layer_with_an_animated_position_is_left_alone_by_canvas_size() {
    let scene = concat!(
        "palette \"p\" {\n    ramp \"ink\" \"#000000\" \"#ffffff\"\n}\n",
        "scene \"s\" palette=\"p\" width=64 height=48 fps=1 duration=\"1f\" background=\"ink.0\" {\n",
        "    layer \"moving\" {\n",
        "        pixels \"art\" x=\"=16\" y=0 {\n",
        "            legend { a \"ink.1\"; }\n",
        "            cel \"\"\"\n            aaaaaaaaaaaaaaaa\n            \"\"\"\n",
        "        }\n    }\n}\n"
    );
    let mut files = common::MemoryFiles::default();
    files
        .files
        .insert(std::path::PathBuf::from("moving.kdl"), scene.to_owned());
    let mut app = App::new(px_editor::assets::ThemeChoice::Dark).unwrap();
    app.open(std::path::Path::new("moving.kdl"), None, &mut files)
        .unwrap();
    let main = app.window_ids().next().unwrap();
    assert_eq!(app.document().unwrap().errors(), [] as [String; 0]);
    let art = tree::elements(app.document().unwrap().source())
        .into_iter()
        .find(|element| element.kind == ElementKind::Pixels)
        .unwrap()
        .path;
    let before = app.document().unwrap().source().node_text(&art).unwrap();

    app.dispatch(
        Intent::ResizeCanvas {
            width: 48,
            height: 48,
            anchor: Anchor::Right,
        },
        main,
        &mut files,
    );
    assert_eq!(size(&app), (48, 48));
    assert_eq!(app.document().unwrap().errors(), [] as [String; 0]);
    assert_eq!(
        app.document().unwrap().source().node_text(&art).unwrap(),
        before,
        "the cels of a layer without a fixed origin are not cropped"
    );
}
