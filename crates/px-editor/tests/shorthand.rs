#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use common::{FILE, MemoryFiles, SIZE, stroke};
use px_core::{Frame, hash_cells};
use px_editor::app::{App, Intent, WindowId};
use px_editor::assets::ThemeChoice;
use px_editor::domain::frames::visible_scene;
use px_editor::domain::{ElementKind, tree};
use px_editor::tools::ToolKind;
use px_engine::{FrameBuffer, Scene};
use px_format::edit::NodePath;
use px_ui::Input;

const SCENE: &str = r##"palette "p" { ramp "sky" "#000000" "#111111"; ramp "ink" "#ffffff" "#eeeeee"; ramp "red" "#ff0000" "#aa0000" }
motif post x {
    rect $x 0 2 8 paint=ink
    rect "=$x+2" 0 2 8 paint=red
}
motif tinted x paint="ink" {
    rect $x 20 2 2 paint=$paint
}
scene "main" palette="p" width=32 height=32 fps=4 duration="2s" loop=#true background="sky" {
    layer "top" {
        rect "=$k*4" 10 2 2 paint=ink repeat=3
        rect "named" 1 14 3 3 paint=red
        post 12
        tinted 20
    }
}
"##;

fn opened(text: &str) -> (App, MemoryFiles, WindowId) {
    let mut files = MemoryFiles::default();
    files.files.insert(PathBuf::from(FILE), text.to_owned());
    let mut app = App::new(ThemeChoice::Dark).unwrap();
    app.open(Path::new(FILE), None, &mut files).unwrap();
    let main = app.window_ids().next().unwrap();
    app.frame(main, &Input::default(), SIZE, &mut files);
    (app, files, main)
}

fn path_of(app: &App, needle: &str) -> NodePath {
    let source = app.document().unwrap().source();
    let offset = source.text().find(needle).unwrap();
    source.path_at(offset).unwrap()
}

fn text(app: &App) -> String {
    app.document().unwrap().text().to_owned()
}

fn changed_lines(before: &str, after: &str) -> Vec<String> {
    before
        .lines()
        .zip(after.lines())
        .filter(|(a, b)| a != b)
        .map(|(_, b)| b.trim().to_owned())
        .collect()
}

fn pick(app: &mut App, files: &mut MemoryFiles, main: WindowId, at: (i32, i32)) -> usize {
    app.dispatch(Intent::Tool(ToolKind::Pick), main, files);
    stroke(app, files, main, &[at]);
    app.editing().picked.unwrap()
}

fn red_paint(app: &App) -> px_core::Paint {
    let loaded = app.document().unwrap().loaded().unwrap();
    loaded
        .scene
        .shapes()
        .iter()
        .map(|shape| shape.paint)
        .find(|paint| {
            loaded
                .paint_names
                .paint(*paint)
                .is_some_and(|name| name.starts_with("red"))
        })
        .unwrap()
}

fn hash(scene: &Scene) -> u64 {
    let mut buffer = FrameBuffer::new(scene);
    hash_cells(buffer.render(scene, Frame::new(0)).unwrap().cells())
}

#[test]
fn picking_any_copy_resolves_to_its_line_and_says_how_many_change() {
    let (mut app, mut files, main) = opened(SCENE);
    for (x, _) in [(0, 0), (4, 1), (8, 2)] {
        let shape = pick(&mut app, &mut files, main, (x, 10));
        let loaded = app.document().unwrap().loaded().unwrap();
        let offset = loaded.origins[shape].offset;
        assert_eq!(offset, SCENE.find("rect \"=$k*4\"").unwrap());
    }
    assert!(app.editing().status.text.contains("changes 3 copies"));
    pick(&mut app, &mut files, main, (2, 15));
    assert!(!app.editing().status.text.contains("copies"));
}

#[test]
fn picking_a_motif_shape_resolves_to_the_call() {
    let (mut app, mut files, main) = opened(SCENE);
    let shape = pick(&mut app, &mut files, main, (15, 3));
    let loaded = app.document().unwrap().loaded().unwrap();
    assert_eq!(loaded.origins[shape].offset, SCENE.find("post 12").unwrap());
}

#[test]
fn recoloring_a_repeat_changes_one_line() {
    let (mut app, mut files, main) = opened(SCENE);
    pick(&mut app, &mut files, main, (4, 10));
    let red = red_paint(&app);
    let before = text(&app);
    app.dispatch(Intent::Swatch(red), main, &mut files);
    let changed = changed_lines(&before, &text(&app));
    assert_eq!(changed.len(), 1, "{changed:?}");
    assert!(changed[0].contains("repeat=3"), "{changed:?}");
    assert!(changed[0].contains("paint=\"red"), "{changed:?}");
}

#[test]
fn a_motif_call_without_a_paint_param_refuses_recolor() {
    let (mut app, mut files, main) = opened(SCENE);
    pick(&mut app, &mut files, main, (12, 3));
    let red = red_paint(&app);
    let before = text(&app);
    app.dispatch(Intent::Swatch(red), main, &mut files);
    assert_eq!(text(&app), before);
    assert!(
        app.editing()
            .status
            .text
            .contains("`post` sets its own colors")
    );
}

#[test]
fn a_motif_call_with_a_paint_param_recolors_on_the_call() {
    let (mut app, mut files, main) = opened(SCENE);
    pick(&mut app, &mut files, main, (20, 20));
    let red = red_paint(&app);
    let before = text(&app);
    app.dispatch(Intent::Swatch(red), main, &mut files);
    let changed = changed_lines(&before, &text(&app));
    assert_eq!(changed.len(), 1, "{changed:?}");
    assert!(
        changed[0].starts_with("tinted 20 paint=\"red"),
        "{changed:?}"
    );
    assert_eq!(app.document().unwrap().errors().len(), 0);
}

#[test]
fn hiding_a_layer_hides_repeat_and_motif_shapes() {
    let (app, _, _) = opened(SCENE);
    let document = app.document().unwrap();
    let shown = visible_scene(document, &BTreeSet::new()).unwrap();
    let hidden = visible_scene(document, &BTreeSet::from(["top".to_owned()])).unwrap();
    assert_ne!(hash(&shown), hash(&hidden));
    let empty = SCENE
        .replace("rect \"=$k*4\" 10 2 2 paint=ink repeat=3", "")
        .replace("rect \"named\" 1 14 3 3 paint=red", "")
        .replace("post 12", "")
        .replace("tinted 20", "");
    let (blank, _, _) = opened(&empty);
    let blank_scene = blank.document().unwrap().scene().unwrap();
    assert_eq!(hash(&hidden), hash(blank_scene));
}

#[test]
fn a_positional_write_hits_the_right_argument_past_a_leading_id() {
    let (mut app, mut files, main) = opened(SCENE);
    let path = path_of(&app, "rect \"named\"");
    app.dispatch(
        Intent::SetArgument {
            path,
            index: 2,
            text: "19".to_owned(),
        },
        main,
        &mut files,
    );
    assert!(text(&app).contains("rect \"named\" 1 19 3 3 paint=red"));
    assert_eq!(app.document().unwrap().errors().len(), 0);
}

#[test]
fn writing_a_number_over_an_expression_keeps_the_node_and_its_selection() {
    let (mut app, mut files, main) = opened(SCENE);
    let path = path_of(&app, "rect \"=$k*4\"");
    app.dispatch(Intent::SelectElement(Some(path.clone())), main, &mut files);
    app.dispatch(
        Intent::SetArgument {
            path,
            index: 0,
            text: "5".to_owned(),
        },
        main,
        &mut files,
    );
    assert!(text(&app).contains("rect 5 10 2 2 paint=ink repeat=3"));
    let selected = app.window(main).unwrap().selected.clone().unwrap();
    let source = app.document().unwrap().source();
    assert!(
        source
            .node_text(&selected)
            .unwrap()
            .starts_with("rect 5 10")
    );
}

#[test]
fn a_plain_string_in_the_first_slot_is_refused_and_an_expression_is_quoted() {
    let (mut app, mut files, main) = opened(SCENE);
    let path = path_of(&app, "rect \"=$k*4\"");
    let before = text(&app);
    app.dispatch(
        Intent::SetArgument {
            path: path.clone(),
            index: 0,
            text: "hello".to_owned(),
        },
        main,
        &mut files,
    );
    assert_eq!(text(&app), before);
    assert!(
        app.editing()
            .status
            .text
            .contains("number or an =expression")
    );
    app.dispatch(
        Intent::SetArgument {
            path,
            index: 0,
            text: "=$k*6".to_owned(),
        },
        main,
        &mut files,
    );
    assert!(text(&app).contains("rect \"=$k*6\" 10 2 2 paint=ink repeat=3"));
}

#[test]
fn changing_repeat_and_undoing_forget_a_stale_pick() {
    let (mut app, mut files, main) = opened(SCENE);
    pick(&mut app, &mut files, main, (8, 10));
    let path = path_of(&app, "rect \"=$k*4\"");
    app.dispatch(
        Intent::SetProperty {
            path,
            key: "repeat".to_owned(),
            value: kdl::KdlValue::Integer(2),
        },
        main,
        &mut files,
    );
    app.frame(main, &Input::default(), SIZE, &mut files);
    assert_eq!(app.editing().picked, None);

    pick(&mut app, &mut files, main, (4, 10));
    app.dispatch(Intent::Undo, main, &mut files);
    app.frame(main, &Input::default(), SIZE, &mut files);
    assert_eq!(app.editing().picked, None);
}

#[test]
fn a_recolor_keeps_the_pick_when_only_values_change() {
    let (mut app, mut files, main) = opened(SCENE);
    let shape = pick(&mut app, &mut files, main, (4, 10));
    let red = red_paint(&app);
    app.dispatch(Intent::Swatch(red), main, &mut files);
    app.frame(main, &Input::default(), SIZE, &mut files);
    assert_eq!(app.editing().picked, Some(shape));
}

#[test]
fn a_motif_definition_lists_as_other_and_its_calls_are_elements() {
    let (app, _, _) = opened(SCENE);
    let elements = tree::elements(app.document().unwrap().source());
    let motif = elements
        .iter()
        .find(|element| element.label == "motif \"tinted\"")
        .unwrap();
    assert_eq!(motif.kind, ElementKind::Other);
    assert!(elements.iter().any(|element| element.label == "post"));
}
