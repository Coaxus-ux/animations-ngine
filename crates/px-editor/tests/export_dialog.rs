#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use std::path::{Path, PathBuf};

use common::{MemoryFiles, SIZE, open};
use px_editor::app::{App, DialogKind, Intent, WindowId, keys};
use px_editor::domain::export::{ExportJob, ExportKind, FrameScope};
use px_ui::{Id, Input, Key};

fn export(app: &mut App, files: &mut MemoryFiles, main: WindowId, kind: ExportKind, out: &str) {
    app.dispatch(
        Intent::Export(ExportJob {
            kind,
            scale: 1,
            transparent: true,
            frames: FrameScope::All,
            out: PathBuf::from(out),
            ..ExportJob::default()
        }),
        main,
        files,
    );
}

fn press(app: &mut App, files: &mut MemoryFiles, main: WindowId, key: Key) {
    let input = Input {
        keys: vec![key],
        ..Input::default()
    };
    app.frame(main, &input, SIZE, files);
    app.frame(main, &Input::default(), SIZE, files);
}

fn field(name: &str) -> Id {
    Id::new("export").with_name(name)
}

fn kind_choice(app: &App, main: WindowId) -> usize {
    app.window(main)
        .unwrap()
        .dialog
        .as_ref()
        .unwrap()
        .form
        .choices[keys::KIND]
}

#[test]
fn export_sprite_sheet_opens_on_the_sheet_kind_after_a_gif_export() {
    let (mut app, mut files, main) = open();
    export(&mut app, &mut files, main, ExportKind::Gif, "out/room.gif");
    app.dispatch(Intent::ShowDialog(DialogKind::Export), main, &mut files);
    assert_eq!(kind_choice(&app, main), ExportKind::Gif.index());
    app.dispatch(
        Intent::ShowDialog(DialogKind::ExportSheet),
        main,
        &mut files,
    );
    assert_eq!(kind_choice(&app, main), ExportJob::default().kind.index());
    let dialog = app.window(main).unwrap().dialog.as_ref().unwrap();
    assert_eq!(dialog.kind, DialogKind::Export);
}

#[test]
fn png_exports_the_current_frame_by_default() {
    let (mut app, mut files, main) = open();
    export(&mut app, &mut files, main, ExportKind::Png, "out/frame.png");
    assert!(files.bytes.is_empty());
    app.dispatch(Intent::Frame(4), main, &mut files);
    app.dispatch(Intent::ShowDialog(DialogKind::Export), main, &mut files);
    let form = &app.window(main).unwrap().dialog.as_ref().unwrap().form;
    assert_eq!(form.choices[keys::FRAMES], 0);
    press(&mut app, &mut files, main, Key::Enter);
    assert!(app.window(main).unwrap().dialog.is_none());
    let from_dialog = files.bytes[Path::new("room.png")].clone();

    app.dispatch(
        Intent::Export(ExportJob {
            kind: ExportKind::Png,
            scale: 1,
            transparent: true,
            frames: FrameScope::Current(4),
            out: PathBuf::from("out/fourth.png"),
            ..ExportJob::default()
        }),
        main,
        &mut files,
    );
    assert_eq!(from_dialog, files.bytes[Path::new("out/fourth.png")]);
    app.dispatch(
        Intent::Export(ExportJob {
            kind: ExportKind::Png,
            scale: 1,
            transparent: true,
            frames: FrameScope::Current(0),
            out: PathBuf::from("out/first.png"),
            ..ExportJob::default()
        }),
        main,
        &mut files,
    );
    assert_ne!(from_dialog, files.bytes[Path::new("out/first.png")]);
}

#[test]
fn gif_honours_the_frame_scope_and_keeps_transparency() {
    let (mut app, mut files, main) = open();
    export(&mut app, &mut files, main, ExportKind::Gif, "out/room.gif");
    app.dispatch(Intent::Frame(4), main, &mut files);
    app.dispatch(Intent::ShowDialog(DialogKind::Export), main, &mut files);
    let form = &app.window(main).unwrap().dialog.as_ref().unwrap().form;
    assert!(form.flags[keys::TRANSPARENT]);
    press(&mut app, &mut files, main, Key::Enter);
    let whole = files.bytes[Path::new("room.gif")].clone();

    app.dispatch(Intent::ShowDialog(DialogKind::Export), main, &mut files);
    let form = &app.window(main).unwrap().dialog.as_ref().unwrap().form;
    assert!(
        form.flags[keys::TRANSPARENT],
        "a GIF keeps the transparent flag"
    );
    assert_eq!(form.choices[keys::FRAMES], 0);

    app.dispatch(
        Intent::Export(ExportJob {
            kind: ExportKind::Gif,
            scale: 1,
            transparent: false,
            frames: FrameScope::Current(4),
            out: PathBuf::from("out/one.gif"),
            ..ExportJob::default()
        }),
        main,
        &mut files,
    );
    app.dispatch(Intent::ShowDialog(DialogKind::Export), main, &mut files);
    let form = &app.window(main).unwrap().dialog.as_ref().unwrap().form;
    assert_eq!(form.choices[keys::FRAMES], 1);
    press(&mut app, &mut files, main, Key::Enter);
    let one = &files.bytes[Path::new("room.gif")];
    assert!(one.len() < whole.len(), "{} < {}", one.len(), whole.len());
}

#[test]
fn tab_visits_only_the_fields_the_kind_shows() {
    let (mut app, mut files, main) = open();
    export(&mut app, &mut files, main, ExportKind::Gif, "out/room.gif");
    let gif = tab_stops(&mut app, &mut files, main);
    assert!(gif.contains(&field(keys::SCALE)) && gif.contains(&field(keys::OUT)));
    assert!(!gif.contains(&field(keys::BORDER)) && !gif.contains(&field(keys::SHAPE)));
    let scale = gif.iter().position(|id| *id == field(keys::SCALE));
    let out = gif.iter().position(|id| *id == field(keys::OUT));
    assert!(scale < out, "Tab follows the dialog from top to bottom");

    export(
        &mut app,
        &mut files,
        main,
        ExportJob::default().kind,
        "out/room.png",
    );
    let png = tab_stops(&mut app, &mut files, main);
    let at = |name: &str| png.iter().position(|id| *id == field(name)).unwrap();
    assert!(at(keys::BORDER) < at(keys::SHAPE) && at(keys::SHAPE) < at(keys::SCALE));
}

fn tab_stops(app: &mut App, files: &mut MemoryFiles, main: WindowId) -> Vec<Id> {
    app.dispatch(Intent::ShowDialog(DialogKind::Export), main, files);
    app.frame(main, &Input::default(), SIZE, files);
    let mut stops = Vec::new();
    loop {
        press(app, files, main, Key::Tab);
        let ui = &app.window(main).unwrap().ui;
        let stop = ui.key_focus.unwrap();
        if stops.contains(&stop) {
            break;
        }
        if ui.focus.is_some() {
            assert_eq!(
                ui.focus,
                Some(stop),
                "a field reached with Tab takes typing"
            );
        }
        stops.push(stop);
    }
    press(app, files, main, Key::Escape);
    stops
}
