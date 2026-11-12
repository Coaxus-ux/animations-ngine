#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use common::open;
use px_core::Rgb;
use px_editor::app::{App, Intent};
use px_editor::domain::palette_model::PaletteModel;
use px_format::edit::NodePath;

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

fn changed_lines(before: &str, after: &str) -> (Vec<String>, Vec<String>) {
    let added = after
        .lines()
        .filter(|line| !before.lines().any(|old| old == *line))
        .map(str::to_owned)
        .collect();
    let removed = before
        .lines()
        .filter(|line| !after.lines().any(|new| new == *line))
        .map(str::to_owned)
        .collect();
    (added, removed)
}

#[test]
fn adding_a_ramp_inserts_one_line_in_the_palette_node() {
    let (mut app, mut files, main) = open();
    let before = text(&app);
    app.dispatch(
        Intent::AddRamp {
            name: "sky".to_owned(),
            colors: vec![SKY],
        },
        main,
        &mut files,
    );
    let after = text(&app);
    assert_eq!(after.lines().count(), before.lines().count() + 1);
    let (added, removed) = changed_lines(&before, &after);
    assert_eq!(added, vec!["    ramp \"sky\" \"102030\"".to_owned()]);
    assert!(removed.is_empty(), "{removed:?}");
    assert_eq!(ramp_colors(&app, "sky"), vec![SKY]);
    assert_eq!(app.editing().status.text, "Done: add ramp");
}

#[test]
fn a_taken_ramp_name_gets_a_numbered_suffix() {
    let (mut app, mut files, main) = open();
    app.dispatch(
        Intent::AddRamp {
            name: "wall".to_owned(),
            colors: Vec::new(),
        },
        main,
        &mut files,
    );
    let names: Vec<String> = room_palette(&app)
        .ramps
        .into_iter()
        .map(|ramp| ramp.name)
        .collect();
    assert_eq!(names.last().unwrap(), "wall-2");
    assert_eq!(ramp_colors(&app, "wall-2").len(), 1);
}

#[test]
fn adding_and_removing_a_tone_rewrites_only_that_ramp_line() {
    let (mut app, mut files, main) = open();
    let before = text(&app);
    let mut colors = ramp_colors(&app, "eye");
    assert_eq!(colors.len(), 2);
    colors.push(SKY);
    app.dispatch(
        Intent::SetRamp {
            ramp: "eye".to_owned(),
            name: "eye".to_owned(),
            colors,
        },
        main,
        &mut files,
    );
    let grown = text(&app);
    assert_eq!(grown.lines().count(), before.lines().count());
    let (added, removed) = changed_lines(&before, &grown);
    assert_eq!(
        added,
        vec!["    ramp \"eye\" \"2f6b3a 7fd36b 102030\"".to_owned()]
    );
    assert_eq!(removed.len(), 1);
    assert!(removed[0].contains("ramp \"eye\""), "{removed:?}");
    let mut colors = ramp_colors(&app, "eye");
    colors.pop();
    app.dispatch(
        Intent::SetRamp {
            ramp: "eye".to_owned(),
            name: "eye".to_owned(),
            colors,
        },
        main,
        &mut files,
    );
    let (added, removed) = changed_lines(&grown, &text(&app));
    assert_eq!(added, vec!["    ramp \"eye\" \"2f6b3a 7fd36b\"".to_owned()]);
    assert_eq!(removed.len(), 1);
    assert_eq!(ramp_colors(&app, "eye").len(), 2);
}

#[test]
fn renaming_a_ramp_to_a_taken_name_is_refused() {
    let (mut app, mut files, main) = open();
    let before = text(&app);
    app.dispatch(
        Intent::SetRamp {
            ramp: "eye".to_owned(),
            name: "fur".to_owned(),
            colors: ramp_colors(&app, "eye"),
        },
        main,
        &mut files,
    );
    assert_eq!(text(&app), before);
    assert!(
        app.editing().status.text.contains("already exists"),
        "{}",
        app.editing().status.text
    );
}

#[test]
fn removing_a_referenced_ramp_is_refused_and_names_the_first_user() {
    let (mut app, mut files, main) = open();
    let before = text(&app);
    app.dispatch(Intent::RemoveRamp("wall".to_owned()), main, &mut files);
    assert_eq!(text(&app), before);
    let status = &app.editing().status.text;
    assert!(
        status.contains("\"wall\"") && status.contains("used by"),
        "{status}"
    );
    assert!(status.contains("scene \"pixel-room\""), "{status}");
}

#[test]
fn an_unused_ramp_can_be_removed() {
    let (mut app, mut files, main) = open();
    let before = text(&app);
    app.dispatch(
        Intent::AddRamp {
            name: "spare".to_owned(),
            colors: vec![SKY],
        },
        main,
        &mut files,
    );
    assert_ne!(text(&app), before);
    app.dispatch(Intent::RemoveRamp("spare".to_owned()), main, &mut files);
    assert_eq!(text(&app), before);
    assert_eq!(app.editing().status.text, "Done: remove ramp");
}

#[test]
fn dropping_a_tone_someone_paints_with_is_refused_and_names_the_user() {
    let (mut app, mut files, main) = open();
    let before = text(&app);
    let mut colors = ramp_colors(&app, "fur");
    assert_eq!(colors.len(), 4);
    colors.pop();
    app.dispatch(
        Intent::SetRamp {
            ramp: "fur".to_owned(),
            name: "fur".to_owned(),
            colors,
        },
        main,
        &mut files,
    );
    assert_eq!(text(&app), before);
    let status = &app.editing().status.text;
    assert!(
        status.starts_with("Tone 3 of ramp \"fur\" is used by"),
        "{status}"
    );
    assert!(status.contains("legend code \"d\" in pixels"), "{status}");
}

fn open_twins() -> (App, common::MemoryFiles, px_editor::app::WindowId) {
    let mut files = common::MemoryFiles::default();
    files.files.insert(
        std::path::PathBuf::from("twins.kdl"),
        concat!(
            "palette \"p\" {\n",
            "    ramp \"sky\" \"#102030\" \"#203040\"\n",
            "    ramp \"warm\" \"#302010\" \"#403020\"\n",
            "    ramp \"edge\" \"#000000\"\n",
            "    ramp \"haze\" \"#808080\"\n",
            "    twin \"sky\" warm=\"warm\"\n",
            "}\n\n",
            "scene \"s\" palette=\"p\" width=2 height=2 fps=1 duration=\"1f\" background=\"sky.0\" {\n",
            "    layer \"far\" fog=\"30%\" fog-paint=\"haze.0\" {\n",
            "        rect x=0 y=0 width=1 height=1 paint=\"sky.1\" outline=\"full\" outline-paint=\"edge.0\"\n",
            "    }\n",
            "}\n"
        )
        .to_owned(),
    );
    let mut app = App::new(px_editor::assets::ThemeChoice::Dark).unwrap();
    app.open(std::path::Path::new("twins.kdl"), None, &mut files)
        .unwrap();
    let main = app.window_ids().next().unwrap();
    app.frame(main, &px_ui::Input::default(), common::SIZE, &mut files);
    assert_eq!(app.document().unwrap().errors(), Vec::<String>::new());
    (app, files, main)
}

#[test]
fn removing_a_ramp_a_twin_needs_is_refused() {
    let (mut app, mut files, main) = open_twins();
    let before = text(&app);
    app.dispatch(Intent::RemoveRamp("warm".to_owned()), main, &mut files);
    assert_eq!(text(&app), before);
    assert_eq!(
        app.editing().status.text,
        "Ramp \"warm\" is used by light color \"warm\" of twin \"sky\""
    );
    app.dispatch(
        Intent::SetRamp {
            ramp: "warm".to_owned(),
            name: "warm".to_owned(),
            colors: vec![SKY],
        },
        main,
        &mut files,
    );
    assert_eq!(text(&app), before);
    assert_eq!(
        app.editing().status.text,
        "Ramp \"warm\" and its twin \"sky\" must have the same number of tones"
    );
    app.dispatch(
        Intent::SetRamp {
            ramp: "sky".to_owned(),
            name: "sky".to_owned(),
            colors: vec![SKY, SKY, SKY],
        },
        main,
        &mut files,
    );
    assert_eq!(text(&app), before);
    assert!(
        app.editing().status.text.contains("its twin \"warm\""),
        "{}",
        app.editing().status.text
    );
}

#[test]
fn outline_and_fog_paints_count_as_uses() {
    let (mut app, mut files, main) = open_twins();
    let before = text(&app);
    app.dispatch(Intent::RemoveRamp("edge".to_owned()), main, &mut files);
    assert_eq!(
        app.editing().status.text,
        "Ramp \"edge\" is used by outline-paint of rect"
    );
    app.dispatch(Intent::RemoveRamp("haze".to_owned()), main, &mut files);
    assert_eq!(
        app.editing().status.text,
        "Ramp \"haze\" is used by fog-paint of layer \"far\""
    );
    assert_eq!(text(&app), before);
}
