#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use common::{SIZE, click, open};
use px_editor::app::{App, Intent, MIN_COLUMN, WindowId};
use px_editor::ports::MemoryFiles;
use px_ui::{Area, Buttons, HEADER, Input, Rect};

fn canvas(app: &App, main: WindowId) -> Rect {
    app.window(main).unwrap().area.unwrap()
}

fn drag(app: &mut App, files: &mut MemoryFiles, main: WindowId, from: (i32, i32), to: (i32, i32)) {
    let held = Buttons {
        left: true,
        ..Buttons::default()
    };
    let at = |point: (i32, i32), pressed: bool, released: bool| Input {
        pointer: Some(point),
        pointer_anywhere: point,
        down: Buttons {
            left: !released,
            ..Buttons::default()
        },
        pressed: if pressed { held } else { Buttons::default() },
        released: if released { held } else { Buttons::default() },
        ..Input::default()
    };
    app.frame(main, &at(from, true, false), SIZE, files);
    app.frame(main, &at(to, false, false), SIZE, files);
    app.frame(main, &at(to, false, true), SIZE, files);
    app.frame(main, &Input::default(), SIZE, files);
}

#[test]
fn dragging_the_left_grip_widens_the_palette_column() {
    let (mut app, mut files, main) = open();
    let scale = app.scale_for(SIZE.0);
    let before = canvas(&app, main);
    let grip = (before.x - 2 * scale, before.y + before.height / 2);
    drag(
        &mut app,
        &mut files,
        main,
        grip,
        (grip.0 + 40 * scale, grip.1),
    );
    let after = canvas(&app, main);
    assert_eq!(after.x - before.x, 40 * scale);
    drag(
        &mut app,
        &mut files,
        main,
        (after.x - scale, grip.1),
        (0, grip.1),
    );
    let narrowest = canvas(&app, main);
    assert!(narrowest.x >= MIN_COLUMN * scale, "{narrowest:?}");
}

#[test]
fn clicking_the_timeline_title_folds_it_like_an_accordion() {
    let (mut app, mut files, main) = open();
    let scale = app.scale_for(SIZE.0);
    let before = canvas(&app, main);
    let title = (
        before.x + 20 * scale,
        before.bottom() + (1 + HEADER / 2) * scale,
    );
    click(&mut app, &mut files, main, title);
    app.frame(main, &Input::default(), SIZE, &mut files);
    let folded = canvas(&app, main);
    assert!(folded.height > before.height, "{folded:?} vs {before:?}");
    assert!(app.layout_text().contains("panels \"Timeline\""));
    let title = (title.0, folded.bottom() + (1 + HEADER / 2) * scale);
    click(&mut app, &mut files, main, title);
    app.frame(main, &Input::default(), SIZE, &mut files);
    assert_eq!(canvas(&app, main), before);
}

#[test]
fn the_palette_moves_to_the_right_column_and_the_layout_remembers_it() {
    let (mut app, mut files, main) = open();
    let before = canvas(&app, main);
    app.dispatch(Intent::TogglePaletteSide, main, &mut files);
    app.frame(main, &Input::default(), SIZE, &mut files);
    let moved = canvas(&app, main);
    assert!(moved.x < before.x, "{moved:?} vs {before:?}");
    let layout = app.layout_text();
    assert!(layout.contains("palette-right=#true"), "{layout}");

    let (mut restored, mut files, main) = open();
    restored.apply_layout(&layout);
    restored.frame(main, &Input::default(), SIZE, &mut files);
    assert_eq!(canvas(&restored, main), moved);
}
