#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use std::path::{Path, PathBuf};

use common::{MemoryFiles, example, open};
use px_core::Rgb;
use px_editor::app::{App, Intent, PickPurpose, Request};
use px_editor::assets::ThemeChoice;
use px_editor::assets::palettes::PRESETS;
use px_editor::domain::palette_model::PaletteModel;
use px_format::edit::{NodePath, Source};
use px_format::{LoadOptions, MemorySources, load};
use px_ui::Input;

const SKY: Rgb = Rgb::new(0x10, 0x20, 0x30);

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

fn loads_with(palette_text: &str, palette: &str, ramp: &str) -> Result<(), String> {
    let swatch = format!(
        "use \"pal.kdl\"\nscene \"s\" palette={palette:?} width=1 height=1 fps=1 duration=\"1f\" background=\"{ramp}.0\" {{}}\n"
    );
    let mut sources = MemorySources::default()
        .with("pal.kdl", palette_text)
        .with("swatch.kdl", swatch);
    load(
        Path::new("swatch.kdl"),
        &mut sources,
        LoadOptions::default(),
    )
    .map(|_| ())
    .map_err(|error| format!("{error:?}"))
}

#[test]
fn the_palette_saves_as_a_standalone_file_that_loads_through_memory_sources() {
    let (mut app, mut files, main) = open();
    app.dispatch(
        Intent::SavePalette(PathBuf::from("pal.kdl")),
        main,
        &mut files,
    );
    let saved = &files.files[Path::new("pal.kdl")];
    assert!(saved.starts_with("palette \"room\" {\n"), "{saved}");
    assert_eq!(
        saved.lines().filter(|line| line.contains("ramp ")).count(),
        6
    );
    loads_with(saved, "room", "wall").unwrap();
    assert_eq!(
        app.editing().status.text,
        "Saved palette \"room\" to pal.kdl"
    );
}

#[test]
fn every_preset_loads() {
    for (name, preset) in PRESETS {
        let source = Source::parse(preset).unwrap();
        let path = NodePath::root().child("palette", Some(name), 0);
        let model = PaletteModel::from_source(&source, &path).unwrap();
        assert!(!model.ramps.is_empty(), "{name}");
        loads_with(preset, name, &model.ramps[0].name).unwrap();
    }
}

#[test]
fn a_preset_replaces_the_ramps_of_a_palette_nobody_paints_with() {
    let mut files = MemoryFiles::default();
    files.files.insert(
        PathBuf::from("blank.kdl"),
        "palette \"mine\" {\n    ramp \"ink\" \"#000000\"\n}\n\nscene \"s\" palette=\"mine\" width=2 height=2 fps=1 duration=\"1f\" background=\"ink.0\" {}\n".to_owned(),
    );
    let mut app = App::new(ThemeChoice::Dark).unwrap();
    app.open(Path::new("blank.kdl"), None, &mut files).unwrap();
    let main = app.window_ids().next().unwrap();
    app.frame(main, &Input::default(), common::SIZE, &mut files);
    app.dispatch(
        Intent::ApplyPreset("gameboy-4".to_owned()),
        main,
        &mut files,
    );
    assert!(
        app.editing()
            .status
            .text
            .contains("\"ink\" is used by background of scene"),
        "{}",
        app.editing().status.text
    );
    app.dispatch(
        Intent::AddRamp {
            name: "green".to_owned(),
            colors: vec![SKY],
        },
        main,
        &mut files,
    );
    app.dispatch(
        Intent::SetProperty {
            path: NodePath::root().child("scene", Some("s"), 0),
            key: "background".to_owned(),
            value: kdl::KdlValue::String("green.0".to_owned()),
        },
        main,
        &mut files,
    );
    app.dispatch(
        Intent::ApplyPreset("gameboy-4".to_owned()),
        main,
        &mut files,
    );
    let text = text(&app);
    assert!(
        text.starts_with("palette \"mine\" {\n    ramp \"green\" \"0f380f"),
        "{text}"
    );
    assert!(!text.contains("\"ink\""), "{text}");
    no_errors(&app);
}

#[test]
fn generate_ramp_writes_the_requested_number_of_tones() {
    let (mut app, mut files, main) = open();
    app.dispatch(
        Intent::GenerateRamp {
            ramp: "eye".to_owned(),
            base: Rgb::new(0x40, 0xa0, 0x40),
            steps: 5,
            hue_shift: 15.0,
            contrast: 0.4,
            saturation: 0.2,
        },
        main,
        &mut files,
    );
    let colors = ramp_colors(&app, "eye");
    assert_eq!(colors.len(), 5);
    assert_eq!(colors[2], Rgb::new(0x40, 0xa0, 0x40));
    assert!(colors[0].luma() < colors[4].luma());
    no_errors(&app);
}

#[test]
fn a_palette_in_a_used_file_is_refused_with_a_status() {
    let mut files = MemoryFiles::default();
    files
        .files
        .insert(PathBuf::from("koi-pond.kdl"), example("koi-pond.kdl"));
    for part in ["pond-palette", "koi", "lily", "ripple", "frog", "dragonfly"] {
        let relative = format!("parts/pond/{part}.kdl");
        files
            .files
            .insert(PathBuf::from(&relative), example(&relative));
    }
    let mut app = App::new(ThemeChoice::Dark).unwrap();
    app.open(Path::new("koi-pond.kdl"), None, &mut files)
        .unwrap();
    let main = app.window_ids().next().unwrap();
    no_errors(&app);
    let before = text(&app);
    app.dispatch(
        Intent::AddRamp {
            name: "sky".to_owned(),
            colors: vec![SKY],
        },
        main,
        &mut files,
    );
    assert_eq!(text(&app), before);
    assert_eq!(
        app.editing().status.text,
        "Palette \"pond\" lives in another file"
    );
}

#[test]
fn importing_an_empty_path_requests_a_native_dialog() {
    let (mut app, mut files, main) = open();
    app.take_requests();
    app.dispatch(Intent::ImportPalette(PathBuf::new()), main, &mut files);
    app.dispatch(Intent::SavePalette(PathBuf::new()), main, &mut files);
    let requests = app.take_requests();
    assert!(
        matches!(
            &requests[0],
            Request::PickFile {
                window,
                purpose: PickPurpose::ImportPalette,
                name: None,
                ..
            } if *window == main
        ),
        "{requests:?}"
    );
    assert!(
        matches!(
            &requests[1],
            Request::PickFile {
                purpose: PickPurpose::SavePalette,
                name: Some(name),
                ..
            } if name == "room.kdl"
        ),
        "{requests:?}"
    );
    app.dispatch(
        Intent::Picked {
            purpose: PickPurpose::SavePalette,
            path: Some(PathBuf::from("out/room.kdl")),
        },
        main,
        &mut files,
    );
    assert!(files.files.contains_key(Path::new("out/room.kdl")));
}

#[test]
fn edit_mode_draws_the_ramps_and_toggles_back() {
    let (mut app, mut files, main) = open();
    app.dispatch(Intent::TogglePaletteEdit, main, &mut files);
    assert!(app.window(main).unwrap().palette_edit);
    let drawn = app
        .frame(main, &Input::default(), common::SIZE, &mut files)
        .is_some();
    assert!(drawn);
    app.dispatch(Intent::TogglePaletteEdit, main, &mut files);
    assert!(!app.window(main).unwrap().palette_edit);
}

#[test]
fn a_preset_keeps_twins_and_comments() {
    let mut files = MemoryFiles::default();
    files.files.insert(
        PathBuf::from("dusk.kdl"),
        concat!(
            "palette \"mine\" {\n",
            "    // evening mood\n",
            "    ramp \"sky\" \"#000000\" \"#111111\" \"#222222\" \"#333333\"\n",
            "    ramp \"sun\" \"#100000\" \"#211111\" \"#322222\" \"#433333\"\n",
            "    twin \"sky\" warm=\"sun\"\n",
            "}\n\n",
            "scene \"s\" palette=\"mine\" width=2 height=2 fps=1 duration=\"1f\" background=\"sky.0\" {}\n"
        )
        .to_owned(),
    );
    let mut app = App::new(ThemeChoice::Dark).unwrap();
    app.open(Path::new("dusk.kdl"), None, &mut files).unwrap();
    let main = app.window_ids().next().unwrap();
    app.frame(main, &Input::default(), common::SIZE, &mut files);
    app.dispatch(Intent::ApplyPreset("dusk-12".to_owned()), main, &mut files);
    let text = text(&app);
    assert!(
        text.starts_with(
            "palette \"mine\" {\n    // evening mood\n    twin \"sky\" warm=\"sun\"\n    ramp \"sky\" \"1a1430"
        ),
        "{text}"
    );
    assert!(text.contains("    ramp \"ground\""), "{text}");
    assert!(!text.contains("#111111"), "{text}");
    no_errors(&app);
    app.dispatch(
        Intent::ApplyPreset("gameboy-4".to_owned()),
        main,
        &mut files,
    );
    assert_eq!(
        app.editing().status.text,
        "Ramp \"sky\" is used by twin \"sky\" and preset \"gameboy-4\" has no such ramp"
    );
}
