//! Tabs, driven headless: each open document keeps its own view and editing state, and
//! switching tabs shows that document's pixels only.
#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use std::path::{Path, PathBuf};

use common::{SIZE, layer, open, window_point};
use px_editor::app::{App, Button, CodeMode, Intent, WindowId};
use px_editor::domain::Document;
use px_ui::{Buttons, Input};

const SECOND: &str = "tiny.kdl";

/// The room plus a second, 8×8 document in a second tab (the active one).
fn two_tabs() -> (App, common::MemoryFiles, WindowId) {
    let (mut app, mut files, main) = open();
    files
        .files
        .insert(PathBuf::from(SECOND), Document::blank_text("tiny", 8, 8));
    app.dispatch(Intent::OpenFile(PathBuf::from(SECOND)), main, &mut files);
    assert_eq!(app.documents().len(), 2);
    assert_eq!(app.documents().active_index(), Some(1));
    (app, files, main)
}

#[test]
fn two_open_files_keep_their_own_zoom_frame_and_selection() {
    let (mut app, mut files, main) = two_tabs();
    app.dispatch(Intent::ActivateTab(0), main, &mut files);
    app.dispatch(Intent::Frame(3), main, &mut files);
    app.dispatch(
        Intent::Zoom {
            steps: 2,
            at: (400, 300),
        },
        main,
        &mut files,
    );
    app.dispatch(Intent::SelectAll, main, &mut files);
    let room_view = app.window(main).unwrap().view.clone();
    assert_eq!(room_view.frame, 3);
    assert!(!app.editing().selection.is_empty());

    app.dispatch(Intent::ActivateTab(1), main, &mut files);
    let tiny_view = app.window(main).unwrap().view.clone();
    assert_eq!(tiny_view.frame, 0);
    assert_ne!(tiny_view.zoom, room_view.zoom);
    assert!(app.editing().selection.is_empty());
    assert_eq!(app.document().unwrap().path(), Path::new(SECOND));

    app.dispatch(Intent::ActivateTab(0), main, &mut files);
    assert_eq!(app.window(main).unwrap().view, room_view);
    assert!(!app.editing().selection.is_empty());
    assert_eq!(app.document().unwrap().path(), Path::new(common::FILE));
}

#[test]
fn switching_tabs_never_shows_the_other_documents_pixels() {
    let (mut app, mut files, main) = two_tabs();
    let tiny = app
        .frame(main, &Input::default(), SIZE, &mut files)
        .unwrap()
        .to_vec();
    app.dispatch(Intent::ActivateTab(0), main, &mut files);
    let room = app
        .frame(main, &Input::default(), SIZE, &mut files)
        .unwrap()
        .to_vec();
    assert_ne!(tiny, room);
    app.dispatch(Intent::ActivateTab(1), main, &mut files);
    let tiny_again = app
        .frame(main, &Input::default(), SIZE, &mut files)
        .unwrap()
        .to_vec();
    assert_eq!(tiny, tiny_again);
}

#[test]
fn closing_the_active_tab_brings_the_neighbours_state_in() {
    let (mut app, mut files, main) = two_tabs();
    app.dispatch(Intent::ActivateTab(0), main, &mut files);
    app.dispatch(Intent::Frame(5), main, &mut files);
    app.dispatch(Intent::ActivateTab(1), main, &mut files);
    app.dispatch(
        Intent::CloseTab {
            index: 1,
            discard: true,
        },
        main,
        &mut files,
    );
    assert_eq!(app.documents().len(), 1);
    assert_eq!(app.document().unwrap().path(), Path::new(common::FILE));
    assert_eq!(app.window(main).unwrap().view.frame, 5);
}

#[test]
fn home_shows_with_tabs_open_and_close_all_with_nothing_dirty_empties_them() {
    let (mut app, mut files, main) = two_tabs();
    app.dispatch(Intent::GoHome, main, &mut files);
    assert!(app.document().is_none() && app.documents().len() == 2);
    app.dispatch(Intent::ActivateTab(1), main, &mut files);
    assert!(app.document().is_some());
    app.dispatch(Intent::CloseAll, main, &mut files);
    assert!(app.documents().is_empty());
    assert!(app.document().is_none());
    assert!(app.window(main).unwrap().dialog.is_none());
    assert!(!app.should_quit());
    app.frame(main, &Input::default(), SIZE, &mut files);
    app.dispatch(Intent::ClearRecent, main, &mut files);
    common::check_golden(&mut app, &mut files, main, "ui-home.txt");
}

/// Clicks the code view (shown alone, so it holds the window's middle) and types `text`.
fn type_into_code(app: &mut App, files: &mut common::MemoryFiles, main: WindowId, text: &str) {
    app.dispatch(Intent::SetCode(CodeMode::Only), main, files);
    app.frame(main, &Input::default(), SIZE, files);
    let middle = (
        i32::try_from(SIZE.0).unwrap() / 2,
        i32::try_from(SIZE.1).unwrap() / 2,
    );
    let press = Input {
        pointer: Some(middle),
        pointer_anywhere: middle,
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
    app.frame(main, &press, SIZE, files);
    let typing = Input {
        text: text.to_owned(),
        ..Input::default()
    };
    app.frame(main, &typing, SIZE, files);
    assert!(
        app.window(main).unwrap().code.edited_at.is_some(),
        "the edit waits for its pause"
    );
}

#[test]
fn a_pending_code_edit_never_lands_in_another_tab() {
    let (mut app, mut files, main) = two_tabs();
    let room = app.documents().get(0).unwrap().document.text().to_owned();
    let tiny = app.document().unwrap().text().to_owned();
    type_into_code(&mut app, &mut files, main, " ");

    let typed = app.window(main).unwrap().code.text.clone();
    assert_ne!(typed, tiny);
    assert_eq!(app.document().unwrap().text(), tiny, "not landed yet");

    app.dispatch(Intent::ActivateTab(0), main, &mut files);
    let settled = Input {
        time_ms: 10_000,
        ..Input::default()
    };
    app.frame(main, &settled, SIZE, &mut files);
    app.frame(main, &settled, SIZE, &mut files);
    assert_eq!(app.document().unwrap().text(), room);
    assert_eq!(app.documents().get(1).unwrap().document.text(), typed);
    assert!(app.window(main).unwrap().selected.is_none());
    app.dispatch(Intent::SetCode(CodeMode::Only), main, &mut files);
    app.frame(main, &settled, SIZE, &mut files);
    assert_eq!(
        app.window(main).unwrap().code.text,
        room,
        "shows its own text"
    );
}

#[test]
fn a_stroke_in_progress_is_dropped_when_the_tab_changes() {
    let (mut app, mut files, main) = two_tabs();
    app.dispatch(Intent::ActivateTab(0), main, &mut files);
    let cat = layer(&app, "cat");
    app.dispatch(Intent::SelectLayer(cat), main, &mut files);
    app.dispatch(
        Intent::Tool(px_editor::tools::ToolKind::Pencil),
        main,
        &mut files,
    );
    app.frame(main, &Input::default(), SIZE, &mut files);
    let room = app.document().unwrap().text().to_owned();
    let tiny = app.documents().get(1).unwrap().document.text().to_owned();
    let at = window_point(&app, main, (2, 2));
    app.dispatch(
        Intent::CanvasPress {
            at,
            button: Button::Primary,
        },
        main,
        &mut files,
    );
    app.dispatch(
        Intent::CanvasDrag {
            at: (at.0 + 20, at.1),
        },
        main,
        &mut files,
    );

    app.dispatch(Intent::ActivateTab(1), main, &mut files);
    app.dispatch(
        Intent::CanvasRelease {
            at: (at.0 + 40, at.1),
        },
        main,
        &mut files,
    );
    assert_eq!(app.document().unwrap().text(), tiny);
    assert!(!app.document().unwrap().is_dirty());
    assert_eq!(app.documents().get(0).unwrap().document.text(), room);
    assert!(!app.documents().get(0).unwrap().document.is_dirty());
}

#[test]
fn a_new_tab_starts_with_nothing_hidden() {
    let (mut app, mut files, main) = open();
    app.dispatch(Intent::ToggleHidden("cat".to_owned()), main, &mut files);
    app.dispatch(Intent::ToggleGrid, main, &mut files);
    app.dispatch(Intent::ToggleOnion, main, &mut files);
    assert!(!app.window(main).unwrap().view.hidden.is_empty());
    files
        .files
        .insert(PathBuf::from(SECOND), Document::blank_text("tiny", 8, 8));
    app.dispatch(Intent::OpenFile(PathBuf::from(SECOND)), main, &mut files);
    let view = &app.window(main).unwrap().view;
    assert!(view.hidden.is_empty());
    assert!(!view.grid && !view.onion);

    app.dispatch(Intent::ActivateTab(0), main, &mut files);
    let view = &app.window(main).unwrap().view;
    assert!(view.hidden.contains("cat") && view.grid && view.onion);
}
