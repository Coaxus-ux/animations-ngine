#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use common::{SIZE, check_golden, click, open};
use px_editor::app::{App, CodeMode, Intent, Request, WindowId};
use px_editor::ports::MemoryFiles;
use px_ui::{HEADER, Input, Key, Modifiers};

fn split(app: &mut App, files: &mut MemoryFiles, main: WindowId) {
    app.dispatch(Intent::SetCode(CodeMode::Split), main, files);
    app.frame(main, &Input::default(), SIZE, files);
}

fn header_slot(app: &App, main: WindowId, slot: i32) -> (i32, i32) {
    let scale = app.scale_for(SIZE.0);
    let canvas = app.window(main).unwrap().area.unwrap();
    let (left, top, width) = (canvas.x / scale, canvas.y / scale, canvas.width / scale);
    let header_right = left + 2 * width - 1;
    let x = header_right - HEADER * slot - HEADER / 2;
    let y = top + 1 + HEADER / 2;
    (x * scale, y * scale)
}

fn code_body(app: &App, main: WindowId) -> (i32, i32) {
    let scale = app.scale_for(SIZE.0);
    let canvas = app.window(main).unwrap().area.unwrap();
    (
        canvas.x + canvas.width * 3 / 2,
        canvas.y + (HEADER + 4) * scale,
    )
}

fn wheel_at(at: (i32, i32), wheel: (f32, f32)) -> Input {
    Input {
        pointer: Some(at),
        pointer_anywhere: at,
        wheel,
        ..Input::default()
    }
}

fn code(app: &App, main: WindowId) -> CodeMode {
    app.window(main).unwrap().view.code
}

#[test]
fn the_close_button_hides_the_code_pane() {
    let (mut app, mut files, main) = open();
    split(&mut app, &mut files, main);
    let close = header_slot(&app, main, 0);
    click(&mut app, &mut files, main, close);
    assert_eq!(code(&app, main), CodeMode::Hidden);
}

#[test]
fn the_size_button_maximizes_and_restores_the_code_pane() {
    let (mut app, mut files, main) = open();
    split(&mut app, &mut files, main);
    let size = header_slot(&app, main, 1);
    click(&mut app, &mut files, main, size);
    assert_eq!(code(&app, main), CodeMode::Only);
    click(&mut app, &mut files, main, size);
    assert_eq!(code(&app, main), CodeMode::Split);
}

#[test]
fn hiding_the_code_releases_keyboard_focus() {
    let (mut app, mut files, main) = open();
    split(&mut app, &mut files, main);
    let body = code_body(&app, main);
    click(&mut app, &mut files, main, body);
    assert!(app.window(main).unwrap().ui.focus.is_some());
    app.dispatch(Intent::SetCode(CodeMode::Hidden), main, &mut files);
    assert!(app.window(main).unwrap().ui.focus.is_none());
}

#[test]
fn wheel_scrolling_the_focused_code_does_not_snap_back_to_the_cursor() {
    let (mut app, mut files, main) = open();
    split(&mut app, &mut files, main);
    let body = code_body(&app, main);
    click(&mut app, &mut files, main, body);
    app.frame(main, &Input::default(), SIZE, &mut files);
    let start = app.window(main).unwrap().code.scroll;
    app.frame(main, &wheel_at(body, (0.0, -30.0)), SIZE, &mut files);
    let scrolled = app.window(main).unwrap().code.scroll;
    assert!(scrolled > start, "the wheel must scroll the code");
    app.frame(main, &Input::default(), SIZE, &mut files);
    assert_eq!(app.window(main).unwrap().code.scroll, scrolled);
    let keys = Input {
        keys: vec![Key::Right],
        ..Input::default()
    };
    app.frame(main, &keys, SIZE, &mut files);
    assert!(
        app.window(main).unwrap().code.scroll < scrolled,
        "moving the cursor brings it back into view"
    );
}

#[test]
fn small_wheel_deltas_add_up_instead_of_jumping() {
    let (mut app, mut files, main) = open();
    split(&mut app, &mut files, main);
    let body = code_body(&app, main);
    app.frame(main, &wheel_at(body, (0.0, -0.1)), SIZE, &mut files);
    assert_eq!(
        app.window(main).unwrap().code.scroll,
        0,
        "a tiny trackpad delta must not move whole lines"
    );
    for _ in 0..20 {
        app.frame(main, &wheel_at(body, (0.0, -0.1)), SIZE, &mut files);
    }
    let after = app.window(main).unwrap().code.scroll;
    assert!(after > 0 && after < 6, "{after} lines");
}

fn command(keys: &[char], clipboard: Option<&str>) -> Input {
    Input {
        keys: keys.iter().map(|key| Key::Letter(*key)).collect(),
        modifiers: Modifiers {
            command: true,
            ..Modifiers::default()
        },
        clipboard: clipboard.map(str::to_owned),
        ..Input::default()
    }
}

fn focused_code(app: &mut App, files: &mut MemoryFiles, main: WindowId) {
    split(app, files, main);
    let body = code_body(app, main);
    click(app, files, main, body);
    app.take_requests();
}

#[test]
fn select_all_and_copy_puts_the_code_on_the_clipboard() {
    let (mut app, mut files, main) = open();
    focused_code(&mut app, &mut files, main);
    app.frame(main, &command(&['a'], None), SIZE, &mut files);
    app.frame(main, &command(&['c'], None), SIZE, &mut files);
    let text = app.window(main).unwrap().code.text.clone();
    assert!(app.take_requests().contains(&Request::Clipboard(text)));
    assert!(
        app.editing().clipboard.is_none(),
        "copying code must not copy canvas pixels"
    );
}

#[test]
fn paste_replaces_the_selection_and_cut_removes_it() {
    let (mut app, mut files, main) = open();
    focused_code(&mut app, &mut files, main);
    app.frame(main, &command(&['a'], None), SIZE, &mut files);
    app.frame(main, &command(&['v'], Some("// pasted")), SIZE, &mut files);
    assert_eq!(app.window(main).unwrap().code.text, "// pasted");
    let shift_home = Input {
        keys: vec![Key::Home],
        modifiers: Modifiers {
            shift: true,
            ..Modifiers::default()
        },
        ..Input::default()
    };
    app.frame(main, &shift_home, SIZE, &mut files);
    app.frame(main, &command(&['x'], None), SIZE, &mut files);
    assert_eq!(app.window(main).unwrap().code.text, "");
    assert!(
        app.take_requests()
            .contains(&Request::Clipboard("// pasted".to_owned()))
    );
}

#[test]
fn typing_over_a_shift_selection_replaces_it() {
    let (mut app, mut files, main) = open();
    focused_code(&mut app, &mut files, main);
    app.frame(main, &command(&['a'], None), SIZE, &mut files);
    app.frame(main, &command(&['v'], Some("abc")), SIZE, &mut files);
    let shift_left = Input {
        keys: vec![Key::Left, Key::Left],
        modifiers: Modifiers {
            shift: true,
            ..Modifiers::default()
        },
        ..Input::default()
    };
    app.frame(main, &shift_left, SIZE, &mut files);
    let typed = Input {
        text: "Z".to_owned(),
        ..Input::default()
    };
    app.frame(main, &typed, SIZE, &mut files);
    assert_eq!(app.window(main).unwrap().code.text, "aZ");
}

#[test]
fn code_only_shows_a_preview_of_the_scene() {
    let (mut app, mut files, main) = open();
    app.dispatch(Intent::SetCode(CodeMode::Only), main, &mut files);
    check_golden(&mut app, &mut files, main, "ui-code-only.txt");
}
