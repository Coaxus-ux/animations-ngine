#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use std::path::PathBuf;

use common::open;
use px_core::Rgb;
use px_editor::app::{App, Intent};
use px_editor::domain::palette_model::PaletteModel;
use px_format::edit::NodePath;

fn text(app: &App) -> String {
    app.document().unwrap().text().to_owned()
}

fn room_palette(app: &App) -> PaletteModel {
    let path = NodePath::root().child("palette", Some("room"), 0);
    PaletteModel::from_source(app.document().unwrap().source(), &path).unwrap()
}

fn ramp_colors(app: &App, name: &str) -> Vec<Rgb> {
    room_palette(app)
        .ramps
        .into_iter()
        .find(|ramp| ramp.name == name)
        .unwrap()
        .colors
}

fn no_errors(app: &App) {
    assert_eq!(app.document().unwrap().errors(), Vec::<String>::new());
}

const SHUFFLED_HEX: &str = concat!(
    "; Lospec\n",
    "3050c0\n",
    "c02020\n",
    "7f7f7f\n",
    "102040\n",
    "208030\n",
    "ff8080\n",
    "6080e0\n",
    "60c060\n",
    "601010\n",
    "a0c0ff\n",
);

#[test]
fn a_lospec_hex_file_imports_as_ramps_clustered_by_hue() {
    let (mut app, mut files, main) = open();
    files
        .files
        .insert(PathBuf::from("colors.hex"), SHUFFLED_HEX.to_owned());
    let before = room_palette(&app).ramps.len();
    app.dispatch(
        Intent::ImportPalette(PathBuf::from("colors.hex")),
        main,
        &mut files,
    );
    let names: Vec<String> = room_palette(&app)
        .ramps
        .into_iter()
        .skip(before)
        .map(|ramp| ramp.name)
        .collect();
    assert_eq!(names, ["red", "green", "blue", "gray"]);
    assert_eq!(
        ramp_colors(&app, "red"),
        vec![
            Rgb::new(0x60, 0x10, 0x10),
            Rgb::new(0xc0, 0x20, 0x20),
            Rgb::new(0xff, 0x80, 0x80)
        ]
    );
    assert_eq!(
        ramp_colors(&app, "blue"),
        vec![
            Rgb::new(0x10, 0x20, 0x40),
            Rgb::new(0x30, 0x50, 0xc0),
            Rgb::new(0x60, 0x80, 0xe0),
            Rgb::new(0xa0, 0xc0, 0xff)
        ]
    );
    assert_eq!(ramp_colors(&app, "gray"), vec![Rgb::new(0x7f, 0x7f, 0x7f)]);
    assert_eq!(app.editing().status, "Imported 4 ramps from colors.hex");
    no_errors(&app);
    app.dispatch(Intent::Undo, main, &mut files);
    assert_eq!(room_palette(&app).ramps.len(), before);
}

#[test]
fn a_family_of_more_than_eight_tones_imports_as_numbered_ramps() {
    let (mut app, mut files, main) = open();
    let hex = (0..10u8)
        .map(|n| format!("10{:02x}10", 0x30 + n * 0x14))
        .collect::<Vec<_>>()
        .join("\n");
    files.files.insert(PathBuf::from("greens.hex"), hex);
    app.dispatch(
        Intent::ImportPalette(PathBuf::from("greens.hex")),
        main,
        &mut files,
    );
    assert_eq!(ramp_colors(&app, "green").len(), 8);
    assert_eq!(
        ramp_colors(&app, "green-2"),
        vec![Rgb::new(0x10, 0xd0, 0x10), Rgb::new(0x10, 0xe4, 0x10)]
    );
    assert_eq!(app.editing().status, "Imported 2 ramps from greens.hex");
    no_errors(&app);
}

#[test]
fn a_gpl_file_imports_as_ramps() {
    let (mut app, mut files, main) = open();
    files.files.insert(
        PathBuf::from("mini.gpl"),
        "GIMP Palette\nName: Mini\nColumns: 2\n#\n0 0 0 black\n255 154 138 salmon\n".to_owned(),
    );
    app.dispatch(
        Intent::ImportPalette(PathBuf::from("mini.gpl")),
        main,
        &mut files,
    );
    assert_eq!(ramp_colors(&app, "red"), vec![Rgb::new(255, 154, 138)]);
    assert_eq!(ramp_colors(&app, "gray"), vec![Rgb::new(0, 0, 0)]);
    assert_eq!(app.editing().status, "Imported 2 ramps from mini.gpl");
}

#[test]
fn an_unreadable_import_leaves_the_document_alone() {
    let (mut app, mut files, main) = open();
    let before = text(&app);
    app.dispatch(
        Intent::ImportPalette(PathBuf::from("missing.hex")),
        main,
        &mut files,
    );
    assert_eq!(text(&app), before);
    assert!(
        app.editing()
            .status
            .starts_with("Could not import missing.hex"),
        "{}",
        app.editing().status
    );
}

#[test]
fn a_gpl_with_a_channels_header_imports() {
    let (mut app, mut files, main) = open();
    files.files.insert(
        PathBuf::from("alpha.gpl"),
        "GIMP Palette\nName: Alpha\nColumns: 1\nChannels: RGBA\n#\n0 0 0 255 black\n".to_owned(),
    );
    app.dispatch(
        Intent::ImportPalette(PathBuf::from("alpha.gpl")),
        main,
        &mut files,
    );
    assert_eq!(ramp_colors(&app, "gray"), vec![Rgb::new(0, 0, 0)]);
    assert_eq!(app.editing().status, "Imported 1 ramp from alpha.gpl");
}
