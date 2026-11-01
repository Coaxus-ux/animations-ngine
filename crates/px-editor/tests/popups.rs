//! Popups over the editor, driven headless: the F1 help and the Export dialogs.
#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use std::path::PathBuf;

use common::{SIZE, check_golden, open};
use px_editor::app::{App, DialogKind, Intent, keys};
use px_editor::domain::export::{ExportJob, ExportKind};
use px_ui::{Area, Buttons, HEADER, Id, Input, ROW, rect};

#[test]
fn the_help_popup_scrolls_and_keeps_close_visible() {
    let (mut app, mut files, main) = open();
    let low = (SIZE.0, 600);
    let scale = app.scale_for(low.0);
    let scroll = Id::new("help").with_name("scroll");
    app.dispatch(Intent::ToggleHelp, main, &mut files);
    app.frame(main, &Input::default(), low, &mut files);
    assert_eq!(app.window(main).unwrap().ui.scroll.get(&scroll), Some(&0));
    let center = (
        i32::try_from(low.0).unwrap() / 2,
        i32::try_from(low.1).unwrap() / 2,
    );
    let wheel = Input {
        pointer: Some(center),
        pointer_anywhere: center,
        wheel: (0.0, -20.0),
        ..Input::default()
    };
    app.frame(main, &wheel, low, &mut files);
    let offset = *app.window(main).unwrap().ui.scroll.get(&scroll).unwrap();
    assert!(
        offset > 0,
        "the help must scroll when the window is too low for it"
    );

    let bounds = rect(
        0,
        0,
        i32::try_from(low.0).unwrap() / scale,
        i32::try_from(low.1).unwrap() / scale,
    );
    let popup = bounds.centered((bounds.width - 40).min(470), bounds.height - 20);
    let close = (
        popup.right() - 1 - 29,
        popup.bottom() - 1 - (ROW + 6) + 2 + i32::midpoint(ROW, 2),
    );
    assert!(close.1 > popup.y + HEADER);
    let press = Input {
        pointer: Some((close.0 * scale, close.1 * scale)),
        pointer_anywhere: (close.0 * scale, close.1 * scale),
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
    app.frame(main, &press, low, &mut files);
    let release = Input {
        pointer: press.pointer,
        pointer_anywhere: press.pointer_anywhere,
        released: Buttons {
            left: true,
            ..Buttons::default()
        },
        ..Input::default()
    };
    app.frame(main, &release, low, &mut files);
    assert!(
        !app.window(main).unwrap().help,
        "Close must stay clickable after scrolling"
    );

    app.dispatch(Intent::ToggleHelp, main, &mut files);
    check_golden(&mut app, &mut files, main, "ui-help.txt");
}

#[test]
fn export_sprite_sheet_opens_on_the_sheet_kind() {
    let (mut app, mut files, main) = open();
    let gif = ExportJob {
        kind: ExportKind::Gif,
        out: PathBuf::from("room.gif"),
        ..ExportJob::default()
    };
    app.dispatch(Intent::Export(gif), main, &mut files);
    let kind = |app: &App| {
        let mut form = app
            .window(main)
            .unwrap()
            .dialog
            .as_ref()
            .unwrap()
            .form
            .clone();
        *form.choice(keys::KIND, usize::MAX)
    };
    app.dispatch(Intent::ShowDialog(DialogKind::Export), main, &mut files);
    assert_eq!(
        kind(&app),
        ExportKind::Gif.index(),
        "Export As keeps the last kind"
    );
    app.dispatch(Intent::CloseDialog, main, &mut files);
    app.dispatch(
        Intent::ShowDialog(DialogKind::ExportSheet),
        main,
        &mut files,
    );
    assert_eq!(kind(&app), ExportJob::default().kind.index());
}
