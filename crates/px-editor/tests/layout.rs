//! The Aseprite arrangement driven headless: shortcuts, the tab strip and the menus.
#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use std::path::{Path, PathBuf};

use common::{MemoryFiles, SIZE, click, example, open, open_empty};
use px_editor::app::{App, DialogKind, Intent, PickPurpose, Request, WindowId};
use px_editor::domain::palette_model::{PaletteModel, palette_path};
use px_editor::views::tab_strip_area;
use px_ui::{Buttons, Input, Key, Modifiers, ROW, rect};

fn keys(keys: &[Key], modifiers: Modifiers) -> Input {
    Input {
        keys: keys.to_vec(),
        modifiers,
        ..Input::default()
    }
}

const COMMAND: Modifiers = Modifiers {
    command: true,
    shift: false,
    ctrl: false,
    alt: false,
};

const COMMAND_SHIFT: Modifiers = Modifiers {
    shift: true,
    ..COMMAND
};

fn dialog(app: &App, window: WindowId) -> Option<DialogKind> {
    app.window(window)
        .unwrap()
        .dialog
        .as_ref()
        .map(|dialog| dialog.kind)
}

#[test]
fn file_shortcuts_open_their_dialogs_and_ctrl_shift_s_is_not_save() {
    let (mut app, mut files, main) = open();
    let writes = files.writes;
    app.take_requests();
    app.frame(
        main,
        &keys(&[Key::Letter('s')], COMMAND_SHIFT),
        SIZE,
        &mut files,
    );
    let requests = app.take_requests();
    assert!(
        requests.iter().any(|request| matches!(
            request,
            Request::PickFile {
                purpose: PickPurpose::SaveAs,
                ..
            }
        )),
        "{requests:?}"
    );
    assert_eq!(files.writes, writes, "Ctrl+Shift+S must not save in place");

    app.frame(main, &keys(&[Key::Letter('n')], COMMAND), SIZE, &mut files);
    assert_eq!(dialog(&app, main), Some(DialogKind::New));
    app.dispatch(px_editor::app::Intent::CloseDialog, main, &mut files);

    app.frame(
        main,
        &keys(&[Key::Letter('c')], Modifiers::default()),
        SIZE,
        &mut files,
    );
    assert_eq!(dialog(&app, main), Some(DialogKind::CanvasSize));
}

/// Window-pixel columns of the tab strip that differ between two drawings of the window.
fn changed_columns(before: &[u32], after: &[u32], scale: i32) -> Vec<i32> {
    let strip = tab_strip_area(rect(
        0,
        0,
        i32::try_from(SIZE.0).unwrap() / scale,
        i32::try_from(SIZE.1).unwrap() / scale,
    ));
    let width = SIZE.0;
    let y = usize::try_from((strip.y + strip.height / 2) * scale).unwrap();
    (0..width)
        .filter(|x| before[y * width + x] != after[y * width + x])
        .map(|x| i32::try_from(x).unwrap())
        .collect()
}

fn second_file(app: &mut App, files: &mut MemoryFiles) -> PathBuf {
    let path = PathBuf::from("hello.kdl");
    files.files.insert(path.clone(), example("hello.kdl"));
    app.open(&path, None, files).unwrap();
    path
}

#[test]
fn the_tab_strip_switches_and_closes_tabs_by_click() {
    let (mut app, mut files, main) = open();
    second_file(&mut app, &mut files);
    assert_eq!(app.documents().active_index(), Some(1));
    let scale = app.scale_for(SIZE.0);
    let strip = tab_strip_area(rect(
        0,
        0,
        i32::try_from(SIZE.0).unwrap() / scale,
        i32::try_from(SIZE.1).unwrap() / scale,
    ));
    let y = (strip.y + strip.height / 2) * scale;
    let second_active = app
        .frame(main, &Input::default(), SIZE, &mut files)
        .unwrap()
        .to_vec();
    app.dispatch(px_editor::app::Intent::ActivateTab(0), main, &mut files);
    let first_active = app
        .frame(main, &Input::default(), SIZE, &mut files)
        .unwrap()
        .to_vec();
    let changed = changed_columns(&second_active, &first_active, scale);
    // Left of the close box, which shows up when the tab is hovered.
    let second_tab_x = *changed.last().unwrap() - 14 * scale;
    assert!(changed.len() > 20, "{changed:?}");

    click(&mut app, &mut files, main, (second_tab_x, y));
    assert_eq!(app.documents().active_index(), Some(1));
    assert_eq!(app.document().unwrap().path(), Path::new("hello.kdl"));

    let middle = Input {
        pointer: Some((second_tab_x, y)),
        pointer_anywhere: (second_tab_x, y),
        down: Buttons {
            middle: true,
            ..Buttons::default()
        },
        pressed: Buttons {
            middle: true,
            ..Buttons::default()
        },
        ..Input::default()
    };
    app.frame(main, &middle, SIZE, &mut files);
    assert_eq!(app.documents().len(), 1, "{}", app.editing().status);
    assert_eq!(app.document().unwrap().path(), Path::new(common::FILE));
}

/// The y (UI pixels) of the File menu's `index`th row, counting separators as rows of
/// their own, with `separators` of them before it.
fn menu_row_y(index: i32, separators: i32) -> i32 {
    let menu_bar = ROW + 3;
    let separator = 5;
    menu_bar + 1 + (index - separators) * (ROW + 2) + separators * separator + i32::midpoint(ROW, 2)
}

#[test]
fn every_menu_entry_has_an_intent_or_is_disabled_without_a_document() {
    let (mut app, mut files, main) = open_empty();
    let scale = app.scale_for(SIZE.0);
    let file_title = (10 * scale, 6 * scale);
    click(&mut app, &mut files, main, file_title);
    assert_eq!(app.window(main).unwrap().ui.open_menu, Some(0));

    let save = (30 * scale, menu_row_y(6, 2) * scale);
    click(&mut app, &mut files, main, save);
    assert_eq!(
        app.window(main).unwrap().ui.open_menu,
        Some(0),
        "a disabled Save must not close the menu"
    );
    assert_eq!(files.writes, 0);
    assert_eq!(dialog(&app, main), None);

    let new = (30 * scale, menu_row_y(0, 0) * scale);
    click(&mut app, &mut files, main, new);
    assert_eq!(app.window(main).unwrap().ui.open_menu, None);
    assert_eq!(dialog(&app, main), Some(DialogKind::New));
}

/// Opens the `index`th menu by clicking File and sliding the pointer right along the bar;
/// returns a window x a few pixels into that menu's title and its dropdown.
fn open_menu(app: &mut App, files: &mut MemoryFiles, main: WindowId, index: usize) -> i32 {
    let scale = app.scale_for(SIZE.0);
    let y = 6 * scale;
    click(app, files, main, (10 * scale, y));
    assert_eq!(app.window(main).unwrap().ui.open_menu, Some(0));
    let mut x = 10 * scale;
    while app.window(main).unwrap().ui.open_menu != Some(index) {
        x += 2 * scale;
        assert!(x < i32::try_from(SIZE.0).unwrap(), "menu {index} not found");
        let hover = Input {
            pointer: Some((x, y)),
            pointer_anywhere: (x, y),
            ..Input::default()
        };
        app.frame(main, &hover, SIZE, files);
    }
    x + 4 * scale
}

const WINDOW_MENU: usize = 7;

#[test]
fn tab_shortcuts_and_menu_entries_agree() {
    let (mut app, mut files, main) = open();
    second_file(&mut app, &mut files);
    let scale = app.scale_for(SIZE.0);
    assert_eq!(app.documents().active_index(), Some(1));
    app.frame(main, &keys(&[Key::Tab], COMMAND), SIZE, &mut files);
    assert_eq!(app.documents().active_index(), Some(0));
    app.frame(main, &keys(&[Key::Tab], COMMAND_SHIFT), SIZE, &mut files);
    assert_eq!(app.documents().active_index(), Some(1));

    let x = open_menu(&mut app, &mut files, main, WINDOW_MENU);
    click(&mut app, &mut files, main, (x, menu_row_y(0, 0) * scale));
    assert_eq!(app.documents().active_index(), Some(0), "Next tab");
    open_menu(&mut app, &mut files, main, WINDOW_MENU);
    click(&mut app, &mut files, main, (x, menu_row_y(1, 0) * scale));
    assert_eq!(app.documents().active_index(), Some(1), "Previous tab");
    open_menu(&mut app, &mut files, main, WINDOW_MENU);
    click(&mut app, &mut files, main, (x, menu_row_y(2, 0) * scale));
    assert_eq!(app.documents().active_index(), None, "Home");
    assert_eq!(app.documents().len(), 2);

    app.frame(main, &keys(&[Key::Tab], COMMAND), SIZE, &mut files);
    assert_eq!(
        app.documents().active_index(),
        None,
        "no tab to cycle from home"
    );
    app.dispatch(Intent::ActivateTab(1), main, &mut files);
    app.frame(main, &keys(&[Key::Home], COMMAND), SIZE, &mut files);
    assert_eq!(app.documents().active_index(), None, "Ctrl+Home");
}

#[test]
fn document_only_shortcuts_do_nothing_on_the_home_tab() {
    let (mut app, mut files, main) = open_empty();
    app.take_requests();
    let status = app.editing().status.clone();
    let command_keys = [
        Key::Letter('e'),
        Key::Letter('p'),
        Key::Letter('a'),
        Key::Letter('v'),
    ];
    for key in command_keys {
        app.frame(main, &keys(&[key], COMMAND), SIZE, &mut files);
    }
    app.frame(
        main,
        &keys(&[Key::Letter('s')], COMMAND_SHIFT),
        SIZE,
        &mut files,
    );
    for key in [Key::Letter('c'), Key::Enter, Key::Delete, Key::Letter('h')] {
        app.frame(main, &keys(&[key], Modifiers::default()), SIZE, &mut files);
    }
    assert_eq!(dialog(&app, main), None);
    let requests = app.take_requests();
    assert!(
        !requests
            .iter()
            .any(|request| matches!(request, Request::PickFile { .. })),
        "{requests:?}"
    );
    assert_eq!(app.editing().status, status);
    app.frame(main, &keys(&[Key::Letter('n')], COMMAND), SIZE, &mut files);
    assert_eq!(dialog(&app, main), Some(DialogKind::New));
}

const PALETTE_MENU: usize = 5;

fn ramps(app: &App) -> Vec<(String, Vec<px_core::Rgb>)> {
    let document = app.document().unwrap();
    let path = palette_path(document).unwrap();
    PaletteModel::from_source(document.source(), &path)
        .unwrap()
        .ramps
        .into_iter()
        .map(|ramp| (ramp.name, ramp.colors))
        .collect()
}

#[test]
fn palette_menu_edits_the_selected_ramp() {
    let (mut app, mut files, main) = open();
    let scale = app.scale_for(SIZE.0);
    let before = ramps(&app);
    let choose = |app: &mut App, files: &mut MemoryFiles, row: i32| {
        let x = open_menu(app, files, main, PALETTE_MENU);
        click(app, files, main, (x, menu_row_y(row, 0) * scale));
    };
    choose(&mut app, &mut files, 4);
    let longer = ramps(&app);
    let changed: Vec<usize> = (0..before.len())
        .filter(|index| before[*index] != longer[*index])
        .collect();
    assert_eq!(changed.len(), 1, "Add a tone changes one ramp: {longer:?}");
    let ramp = changed[0];
    assert_eq!(longer[ramp].1.len(), before[ramp].1.len() + 1);
    choose(&mut app, &mut files, 5);
    assert_eq!(ramps(&app), before, "Drop the last tone undoes it");
    choose(&mut app, &mut files, 3);
    let generated = ramps(&app);
    assert_eq!(generated[ramp].0, before[ramp].0);
    assert_eq!(generated[ramp].1.len(), before[ramp].1.len());
    assert_ne!(
        generated[ramp].1, before[ramp].1,
        "Generate ramp recolors it"
    );
    let status = app.editing().status.clone();
    choose(&mut app, &mut files, 2);
    let after = ramps(&app);
    assert!(
        after.len() == before.len() - 1 || app.editing().status != status,
        "Remove ramp removes it or says who paints with it: {}",
        app.editing().status
    );
}
