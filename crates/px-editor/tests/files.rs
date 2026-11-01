#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use std::path::{Path, PathBuf};

use common::{FILE, SIZE, layer, open, open_empty, stroke};
use px_editor::app::{App, DialogKind, Intent, PickPurpose, Request, keys};
use px_editor::assets::ThemeChoice;
use px_editor::domain::NewSpec;
use px_ui::{Input, Key};

const SECOND: &str = "scenes/second.kdl";

fn hero_spec() -> NewSpec {
    NewSpec {
        name: "hero".to_owned(),
        width: 32,
        height: 24,
        fps: 24,
        frames: 48,
        preset: "basic-16".to_owned(),
        background: None,
    }
}

#[test]
fn the_home_tab_draws_without_a_document() {
    let (mut app, mut files, main) = open_empty();
    common::check_golden(&mut app, &mut files, main, "ui-home.txt");
}

#[test]
fn a_new_document_follows_the_spec_and_is_dirty_until_saved() {
    let (mut app, mut files, main) = open_empty();
    app.dispatch(Intent::NewDocument(hero_spec()), main, &mut files);
    let document = app.document().unwrap();
    let text = document.text();
    assert!(
        text.contains("width=32 height=24 fps=24 duration=\"48f\""),
        "{text}"
    );
    assert!(document.is_dirty() && document.is_untitled());
    assert_eq!(document.path(), Path::new("hero.kdl"));
    app.dispatch(
        Intent::SaveAs(PathBuf::from("art/hero.kdl")),
        main,
        &mut files,
    );
    let document = app.document().unwrap();
    assert_eq!(files.files[Path::new("art/hero.kdl")], document.text());
    assert!(!document.is_dirty() && !document.is_untitled());
    assert_eq!(app.recent().paths(), [PathBuf::from("art/hero.kdl")]);
}

#[test]
fn two_new_documents_get_different_names() {
    let (mut app, mut files, main) = open_empty();
    app.dispatch(Intent::NewDocument(hero_spec()), main, &mut files);
    app.dispatch(Intent::NewDocument(hero_spec()), main, &mut files);
    assert_eq!(app.document().unwrap().path(), Path::new("hero-2.kdl"));
    assert_eq!(app.documents().len(), 2);
}

#[test]
fn saving_an_untitled_document_asks_where() {
    let (mut app, mut files, main) = open_empty();
    app.dispatch(Intent::NewDocument(hero_spec()), main, &mut files);
    app.take_requests();
    app.dispatch(Intent::Save, main, &mut files);
    let requests = app.take_requests();
    assert!(
        requests.iter().any(|request| matches!(
            request,
            Request::PickFile {
                purpose: PickPurpose::SaveAs,
                name: Some(name),
                ..
            } if name == "hero.kdl"
        )),
        "{requests:?}"
    );
    assert!(files.files.is_empty());
}

#[test]
fn ctrl_o_requests_a_native_pick_and_the_picked_path_opens_in_a_new_tab() {
    let (mut app, mut files, main) = open();
    app.take_requests();
    app.dispatch(Intent::OpenFile(PathBuf::new()), main, &mut files);
    let requests = app.take_requests();
    assert!(
        requests.iter().any(|request| matches!(
            request,
            Request::PickFile {
                purpose: PickPurpose::Open,
                ..
            }
        )),
        "{requests:?}"
    );
    files
        .files
        .insert(PathBuf::from(SECOND), common::example("hello.kdl"));
    app.dispatch(
        Intent::Picked {
            purpose: PickPurpose::Open,
            path: Some(PathBuf::from(SECOND)),
        },
        main,
        &mut files,
    );
    assert_eq!(app.documents().len(), 2);
    assert_eq!(app.document().unwrap().path(), Path::new(SECOND));
    assert_eq!(app.recent().paths()[0], PathBuf::from(SECOND));
}

#[test]
fn a_cancelled_pick_changes_nothing() {
    let (mut app, mut files, main) = open();
    let text = app.document().unwrap().text().to_owned();
    app.dispatch(
        Intent::Picked {
            purpose: PickPurpose::Open,
            path: None,
        },
        main,
        &mut files,
    );
    assert_eq!(app.documents().len(), 1);
    assert_eq!(app.document().unwrap().text(), text);
    assert!(app.recent().is_empty());
    assert_eq!(app.editing().status.text, "Cancelled");
}

#[test]
fn a_pick_for_another_purpose_passes_through_the_file_handler() {
    let (mut app, mut files, main) = open();
    app.dispatch(
        Intent::Picked {
            purpose: PickPurpose::ExportOut,
            path: Some(PathBuf::from("out.gif")),
        },
        main,
        &mut files,
    );
    assert_eq!(app.documents().len(), 1);
    assert!(!files.files.contains_key(Path::new("out.gif")));
}

#[test]
fn closing_a_dirty_tab_asks_first_and_then_runs_the_pending_intent() {
    let (mut app, mut files, main) = open();
    let cat = layer(&app, "cat");
    app.dispatch(Intent::SelectLayer(cat), main, &mut files);
    stroke(&mut app, &mut files, main, &[(2, 2), (3, 2)]);
    assert!(app.document().unwrap().is_dirty());
    app.dispatch(
        Intent::CloseTab {
            index: 0,
            discard: false,
        },
        main,
        &mut files,
    );
    let dialog = app.window(main).unwrap().dialog.as_ref().unwrap();
    assert_eq!(dialog.kind, DialogKind::Confirm);
    assert_eq!(dialog.form.message, format!("Save changes to {FILE}?"));
    assert_eq!(app.documents().len(), 1);
    app.dispatch(
        Intent::CloseTab {
            index: 0,
            discard: true,
        },
        main,
        &mut files,
    );
    assert!(app.documents().is_empty());
    assert!(app.document().is_none());
    assert!(!app.should_quit());
}

#[test]
fn quitting_with_a_dirty_tab_asks_and_resumes_quitting_afterwards() {
    let (mut app, mut files, main) = open();
    let cat = layer(&app, "cat");
    app.dispatch(Intent::SelectLayer(cat), main, &mut files);
    stroke(&mut app, &mut files, main, &[(2, 2), (3, 2)]);
    app.dispatch(Intent::Quit, main, &mut files);
    assert!(!app.should_quit());
    let dialog = app.window(main).unwrap().dialog.clone().unwrap();
    assert_eq!(dialog.then.as_deref(), Some(&Intent::Quit));
    app.dispatch(
        Intent::CloseTab {
            index: 0,
            discard: true,
        },
        main,
        &mut files,
    );
    app.dispatch(Intent::Quit, main, &mut files);
    assert!(app.should_quit());
}

fn press(app: &mut App, files: &mut common::MemoryFiles, main: px_editor::app::WindowId, key: Key) {
    let input = Input {
        keys: vec![key],
        ..Input::default()
    };
    app.frame(main, &input, SIZE, files);
}

fn save_as_requests(app: &mut App) -> usize {
    app.take_requests()
        .iter()
        .filter(|request| {
            matches!(
                request,
                Request::PickFile {
                    purpose: PickPurpose::SaveAs,
                    ..
                }
            )
        })
        .count()
}

#[test]
fn confirm_save_on_an_untitled_document_closes_it_after_the_pick() {
    let (mut app, mut files, main) = open_empty();
    app.dispatch(Intent::NewDocument(hero_spec()), main, &mut files);
    app.dispatch(Intent::Quit, main, &mut files);
    assert_eq!(
        app.window(main).unwrap().dialog.as_ref().unwrap().kind,
        DialogKind::Confirm
    );
    app.take_requests();

    press(&mut app, &mut files, main, Key::Enter);
    assert_eq!(save_as_requests(&mut app), 1, "Save asks where to save");
    let dialog = app.window(main).unwrap().dialog.as_ref().unwrap();
    assert_eq!(
        dialog.kind,
        DialogKind::Confirm,
        "the question waits for the pick"
    );
    assert_eq!(app.documents().len(), 1);
    assert!(!app.should_quit());

    app.dispatch(
        Intent::Picked {
            purpose: PickPurpose::SaveAs,
            path: None,
        },
        main,
        &mut files,
    );
    assert!(app.window(main).unwrap().dialog.is_some());
    assert_eq!(app.documents().len(), 1);

    press(&mut app, &mut files, main, Key::Enter);
    assert_eq!(save_as_requests(&mut app), 1);
    app.dispatch(
        Intent::Picked {
            purpose: PickPurpose::SaveAs,
            path: Some(PathBuf::from("art/hero.kdl")),
        },
        main,
        &mut files,
    );
    assert!(files.files.contains_key(Path::new("art/hero.kdl")));
    assert!(app.window(main).unwrap().dialog.is_none());
    assert!(app.documents().is_empty());
    assert!(app.should_quit());
}

#[test]
fn confirm_acts_on_the_tab_it_asked_about_even_after_a_tab_switch() {
    let (mut app, mut files, main) = open();
    let cat = layer(&app, "cat");
    app.dispatch(Intent::SelectLayer(cat), main, &mut files);
    stroke(&mut app, &mut files, main, &[(2, 2), (3, 2)]);
    let edited = app.document().unwrap().text().to_owned();
    files
        .files
        .insert(PathBuf::from(SECOND), common::example("hello.kdl"));
    app.dispatch(Intent::OpenFile(PathBuf::from(SECOND)), main, &mut files);
    app.dispatch(
        Intent::CloseTab {
            index: 0,
            discard: false,
        },
        main,
        &mut files,
    );
    let dialog = app.window(main).unwrap().dialog.as_ref().unwrap();
    assert_eq!(dialog.form.numbers[keys::TAB], 0);
    assert_eq!(app.documents().active_index(), Some(0));

    app.dispatch(Intent::ActivateTab(1), main, &mut files);
    press(&mut app, &mut files, main, Key::Enter);
    assert_eq!(files.files[Path::new(FILE)], edited);
    assert_eq!(app.documents().len(), 1);
    assert_eq!(app.document().unwrap().path(), Path::new(SECOND));
    assert!(app.window(main).unwrap().dialog.is_none());
}

#[test]
fn escape_answers_the_confirm_dialog_with_cancel() {
    let (mut app, mut files, main) = open();
    let cat = layer(&app, "cat");
    app.dispatch(Intent::SelectLayer(cat), main, &mut files);
    stroke(&mut app, &mut files, main, &[(2, 2), (3, 2)]);
    app.dispatch(Intent::Quit, main, &mut files);
    press(&mut app, &mut files, main, Key::Escape);
    assert!(app.window(main).unwrap().dialog.is_none());
    assert_eq!(app.documents().len(), 1);
    assert!(app.document().unwrap().is_dirty());
    assert!(!app.should_quit());
    assert_ne!(files.files[Path::new(FILE)], app.document().unwrap().text());
}

#[test]
fn recent_files_folder_round_trip_through_the_layout_text() {
    let (mut app, mut files, main) = open_empty();
    files
        .files
        .insert(PathBuf::from(SECOND), common::example("hello.kdl"));
    app.dispatch(Intent::OpenFile(PathBuf::from(SECOND)), main, &mut files);
    app.dispatch(
        Intent::OpenFile(PathBuf::from("missing.kdl")),
        main,
        &mut files,
    );
    let status = &app.editing().status;
    assert!(
        status.problem && status.text.ends_with("(missing.kdl)"),
        "{}",
        status.text
    );
    let layout = app.layout_text();
    assert!(layout.contains("folder \"scenes\""), "{layout}");
    let mut restored = App::new(ThemeChoice::Dark).unwrap();
    restored.apply_layout(&layout);
    assert_eq!(restored.recent(), app.recent());
    assert_eq!(restored.layout_text(), layout);
    app.dispatch(
        Intent::ForgetRecent(PathBuf::from(SECOND)),
        main,
        &mut files,
    );
    assert!(app.recent().is_empty());
}

#[test]
fn the_new_dialog_draws_the_same_pixels() {
    let (mut app, mut files, main) = open_empty();
    app.dispatch(Intent::ShowDialog(DialogKind::New), main, &mut files);
    app.frame(main, &Input::default(), SIZE, &mut files);
    assert_eq!(
        app.window(main)
            .unwrap()
            .dialog
            .as_ref()
            .unwrap()
            .form
            .numbers[keys::WIDTH],
        64
    );
    common::check_golden(&mut app, &mut files, main, "ui-dialog-new.txt");
}
