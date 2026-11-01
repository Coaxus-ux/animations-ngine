#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use std::path::{Path, PathBuf};

use common::{SIZE, example, open};
use px_editor::app::{App, CodeMode, Intent};
use px_editor::assets::ThemeChoice;
use px_editor::domain::{ElementKind, tree};
use px_format::edit::NodePath;
use px_ui::{Input, Key};

fn element(app: &App, kind: ElementKind, label: &str) -> NodePath {
    tree::elements(app.document().unwrap().source())
        .into_iter()
        .find(|element| element.kind == kind && element.label == label)
        .map(|element| element.path)
        .unwrap()
}

fn size_of_scene(app: &App) -> (u16, u16) {
    let size = app.document().unwrap().scene().unwrap().size();
    (size.width(), size.height())
}

fn node_text(app: &App, path: &NodePath) -> String {
    app.document().unwrap().source().node_text(path).unwrap()
}

#[test]
fn selecting_an_element_highlights_it_and_the_inspector_shows_its_node() {
    let (mut app, mut files, main) = open();
    let before = app
        .frame(main, &Input::default(), SIZE, &mut files)
        .unwrap()
        .to_vec();
    let room = element(&app, ElementKind::Layer, "layer \"room\"");
    app.dispatch(Intent::SelectElement(Some(room.clone())), main, &mut files);
    assert_eq!(app.window(main).unwrap().selected, Some(room.clone()));
    assert_eq!(app.editing().picked, None, "a layer is not a shape");
    let after = app
        .frame(main, &Input::default(), SIZE, &mut files)
        .unwrap()
        .to_vec();
    assert_ne!(before, after, "the inspector should show the layer now");

    let rect = element(&app, ElementKind::Shape, "rect");
    app.dispatch(Intent::SelectElement(Some(rect.clone())), main, &mut files);
    let picked = app.editing().picked.unwrap();
    let document = app.document().unwrap();
    let (start, _) = document.source().span(&rect).unwrap();
    assert_eq!(document.loaded().unwrap().origins[picked].offset, start);

    app.dispatch(Intent::SelectElement(None), main, &mut files);
    assert_eq!(app.window(main).unwrap().selected, None);
    assert_eq!(app.editing().picked, None);
}

#[test]
fn a_scene_built_from_parts_lists_its_use_nodes_first() {
    let mut files = common::MemoryFiles::default();
    for part in ["pond-palette", "koi", "lily", "ripple", "frog", "dragonfly"] {
        files.files.insert(
            PathBuf::from(format!("parts/pond/{part}.kdl")),
            example(&format!("parts/pond/{part}.kdl")),
        );
    }
    files
        .files
        .insert(PathBuf::from("koi.kdl"), example("koi-pond.kdl"));
    let mut app = App::new(ThemeChoice::Dark).unwrap();
    app.open(Path::new("koi.kdl"), None, &mut files).unwrap();
    let main = app.window_ids().next().unwrap();
    let rows = tree::elements(app.document().unwrap().source());
    assert_eq!(rows[0].kind, ElementKind::Use, "{:?}", rows[0]);
    assert!(rows.iter().any(|row| row.kind == ElementKind::UseScene));
    let first = rows[0].path.clone();
    app.dispatch(Intent::SetCode(CodeMode::Split), main, &mut files);
    app.dispatch(Intent::SelectElement(Some(first.clone())), main, &mut files);
    app.frame(main, &Input::default(), SIZE, &mut files);
    assert_eq!(app.window(main).unwrap().code.text, node_text(&app, &first));
    assert!(app.window(main).unwrap().code.text.starts_with("use "));
}

#[test]
fn selecting_an_element_narrows_the_code_text_to_its_node() {
    let (mut app, mut files, main) = open();
    app.dispatch(Intent::SetCode(CodeMode::Split), main, &mut files);
    app.frame(main, &Input::default(), SIZE, &mut files);
    assert_eq!(
        app.window(main).unwrap().code.text,
        app.document().unwrap().text()
    );
    let cat = element(&app, ElementKind::Layer, "layer \"cat\"");
    app.dispatch(Intent::SelectElement(Some(cat.clone())), main, &mut files);
    app.frame(main, &Input::default(), SIZE, &mut files);
    let code = &app.window(main).unwrap().code;
    assert_eq!(code.text, node_text(&app, &cat));
    assert!(code.text.starts_with("layer \"cat\" {"), "{}", code.text);
    assert_eq!(code.cursor, 0);
}

#[test]
fn replace_node_rewrites_only_that_node_and_keeps_it_selected_after_a_rename() {
    let (mut app, mut files, main) = open();
    let cat = element(&app, ElementKind::Layer, "layer \"cat\"");
    app.dispatch(Intent::SelectElement(Some(cat.clone())), main, &mut files);
    let before = app.document().unwrap().text().to_owned();
    let text = node_text(&app, &cat).replace("layer \"cat\"", "layer \"kitten\"");
    app.dispatch(
        Intent::ReplaceNode {
            path: cat.clone(),
            text,
        },
        main,
        &mut files,
    );
    let after = app.document().unwrap().text().to_owned();
    assert_eq!(before.lines().count(), after.lines().count());
    let changed: Vec<(&str, &str)> = before
        .lines()
        .zip(after.lines())
        .filter(|(a, b)| a != b)
        .collect();
    assert_eq!(changed.len(), 1, "{changed:?}");
    assert_eq!(changed[0].1, "    layer \"kitten\" {");
    assert_eq!(app.document().unwrap().errors(), [] as [String; 0]);

    let kitten = element(&app, ElementKind::Layer, "layer \"kitten\"");
    assert_ne!(kitten, cat);
    assert_eq!(app.window(main).unwrap().selected, Some(kitten));
    assert!(
        app.editing().status.contains("edit code"),
        "{}",
        app.editing().status
    );
}

#[test]
fn bad_node_text_is_refused_and_the_scene_stays() {
    let (mut app, mut files, main) = open();
    let cat = element(&app, ElementKind::Layer, "layer \"cat\"");
    app.dispatch(Intent::SelectElement(Some(cat.clone())), main, &mut files);
    let before = app.document().unwrap().text().to_owned();
    app.dispatch(
        Intent::ReplaceNode {
            path: cat.clone(),
            text: "layer \"cat\" {".to_owned(),
        },
        main,
        &mut files,
    );
    assert_eq!(app.document().unwrap().text(), before);
    assert_eq!(app.document().unwrap().errors(), [] as [String; 0]);
    assert_eq!(size_of_scene(&app), (64, 48));
    assert_eq!(app.window(main).unwrap().selected, Some(cat));
    let status = &app.editing().status;
    assert!(
        !status.is_empty() && !status.starts_with("Done"),
        "{status}"
    );
}

#[test]
fn escape_in_node_mode_returns_to_the_whole_file() {
    let (mut app, mut files, main) = open();
    app.dispatch(Intent::SetCode(CodeMode::Split), main, &mut files);
    let cat = element(&app, ElementKind::Layer, "layer \"cat\"");
    app.dispatch(Intent::SelectElement(Some(cat.clone())), main, &mut files);
    app.frame(main, &Input::default(), SIZE, &mut files);
    assert_eq!(app.window(main).unwrap().code.text, node_text(&app, &cat));
    let escape = Input {
        keys: vec![Key::Escape],
        ..Input::default()
    };
    app.frame(main, &escape, SIZE, &mut files);
    assert_eq!(app.window(main).unwrap().selected, None);
    app.frame(main, &Input::default(), SIZE, &mut files);
    assert_eq!(
        app.window(main).unwrap().code.text,
        app.document().unwrap().text()
    );
}
