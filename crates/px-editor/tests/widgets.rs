//! The dialog and form widgets of px-ui with the editor's own theme: modal dialogs make
//! the rest of the window inert, number fields stay in range, segmented controls and tab
//! strips report what was clicked, and quick second clicks are double clicks.
#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use px_editor::assets::{Assets, ThemeChoice};
use px_ui::{
    Area, Buttons, Id, Input, Key, Rect, Response, Tab, TabAction, Ui, UiCanvas, UiState, button,
    modal, number_field, rect, segmented, tab_strip, text_width,
};

const WINDOW: (i32, i32) = (200, 120);

fn frame<T>(
    assets: &Assets,
    state: &mut UiState,
    input: &Input,
    draw: impl FnOnce(&mut Ui<'_>) -> T,
) -> T {
    let mut canvas = UiCanvas::new(WINDOW.0, WINDOW.1, assets.theme.roles.window);
    let mut ui = Ui::new(
        &mut canvas,
        &assets.theme,
        &assets.font,
        &assets.icons,
        input,
        state,
    );
    let result = draw(&mut ui);
    ui.finish();
    result
}

fn pointer(at: (i32, i32), buttons: impl Fn(bool) -> Buttons, pressed: bool) -> Input {
    Input {
        pointer: Some(at),
        pointer_anywhere: at,
        down: buttons(pressed),
        pressed: buttons(pressed),
        released: buttons(!pressed),
        ..Input::default()
    }
}

fn left(held: bool) -> Buttons {
    Buttons {
        left: held,
        ..Buttons::default()
    }
}

fn middle(held: bool) -> Buttons {
    Buttons {
        middle: held,
        ..Buttons::default()
    }
}

fn click<T>(
    assets: &Assets,
    state: &mut UiState,
    at: (i32, i32),
    mut draw: impl FnMut(&mut Ui<'_>) -> T,
) -> T {
    frame(assets, state, &pointer(at, left, true), &mut draw);
    frame(assets, state, &pointer(at, left, false), &mut draw)
}

fn keys(keys: &[Key], text: &str) -> Input {
    Input {
        keys: keys.to_vec(),
        text: text.to_owned(),
        ..Input::default()
    }
}

#[test]
fn a_modal_makes_widgets_outside_it_inert_in_the_same_frame() {
    let assets = Assets::load(ThemeChoice::Dark).unwrap();
    let mut state = UiState::default();
    let outside = rect(0, 0, 60, 20);
    let mut draw = |ui: &mut Ui<'_>| {
        let outside_clicked = button(ui, Id::new("outside"), outside, "Outside");
        let outside_hovered = ui.hovering(outside);
        let inside_clicked = modal(
            ui,
            Id::new("dialog"),
            "Dialog",
            (100, 60),
            |ui, _header, body| {
                button(
                    ui,
                    Id::new("inside"),
                    rect(body.x, body.y, 40, 12),
                    "Inside",
                )
            },
        );
        (outside_clicked, outside_hovered, inside_clicked)
    };
    state.modal = true;
    let (outside_clicked, outside_hovered, _) = click(&assets, &mut state, (10, 10), &mut draw);
    assert!(!outside_clicked, "the button under the dialog was clicked");
    assert!(!outside_hovered, "the button under the dialog was hovered");
    let (_, _, inside_clicked) = click(&assets, &mut state, (60, 48), &mut draw);
    assert!(inside_clicked, "the dialog's own button was not clicked");
    state.modal = false;
    let (outside_clicked, _, _) = click(&assets, &mut state, (10, 10), &mut draw);
    assert!(outside_clicked);
}

#[test]
fn number_field_clamps_to_its_range() {
    let assets = Assets::load(ThemeChoice::Dark).unwrap();
    let mut state = UiState::default();
    let area: Rect = rect(0, 0, 60, 12);
    let mut value = 8_i64;
    let draw =
        |ui: &mut Ui<'_>, value: &mut i64| number_field(ui, Id::new("count"), area, value, (0, 10));
    for _ in 0..4 {
        click(&assets, &mut state, (54, 6), |ui| draw(ui, &mut value));
    }
    assert_eq!(value, 10);
    let changed = click(&assets, &mut state, (54, 6), |ui| draw(ui, &mut value));
    assert!(!changed);
    click(&assets, &mut state, (30, 6), |ui| draw(ui, &mut value));
    assert_eq!(state.focus, Some(Id::new("count")));
    for input in [keys(&[Key::Backspace, Key::Backspace], ""), keys(&[], "-5")] {
        frame(&assets, &mut state, &input, |ui| draw(ui, &mut value));
    }
    assert_eq!(
        value, 10,
        "typing changed the value before it was committed"
    );
    let changed = frame(&assets, &mut state, &keys(&[Key::Enter], ""), |ui| {
        draw(ui, &mut value)
    });
    assert!(changed);
    assert_eq!(value, 0);
    assert_eq!(state.focus, None);
    for _ in 0..3 {
        click(&assets, &mut state, (6, 6), |ui| draw(ui, &mut value));
    }
    assert_eq!(value, 0);
}

#[test]
fn segmented_switches_on_click() {
    let assets = Assets::load(ThemeChoice::Dark).unwrap();
    let mut state = UiState::default();
    let mut chosen = 0;
    let draw = |ui: &mut Ui<'_>, chosen: &mut usize| {
        segmented(
            ui,
            Id::new("mode"),
            rect(0, 0, 90, 12),
            &["A", "B", "C"],
            chosen,
        )
    };
    let changed = click(&assets, &mut state, (75, 6), |ui| draw(ui, &mut chosen));
    assert!(changed);
    assert_eq!(chosen, 2);
    let changed = click(&assets, &mut state, (75, 6), |ui| draw(ui, &mut chosen));
    assert!(!changed);
    click(&assets, &mut state, (45, 6), |ui| draw(ui, &mut chosen));
    assert_eq!(chosen, 1);
}

#[test]
fn a_second_click_within_400_ms_is_a_double_click() {
    let assets = Assets::load(ThemeChoice::Dark).unwrap();
    let mut state = UiState::default();
    let area = rect(0, 0, 40, 12);
    let draw = |ui: &mut Ui<'_>| -> Response { ui.interact(Id::new("item"), area) };
    let timed_click = |state: &mut UiState, at_ms: u64| {
        let mut press = pointer((5, 5), left, true);
        press.time_ms = at_ms;
        let mut release = pointer((5, 5), left, false);
        release.time_ms = at_ms + 10;
        frame(&assets, state, &press, draw);
        frame(&assets, state, &release, draw)
    };
    let first = timed_click(&mut state, 1000);
    assert!(first.clicked && !first.double_clicked);
    let second = timed_click(&mut state, 1200);
    assert!(second.double_clicked);
    let late = timed_click(&mut state, 2000);
    assert!(late.clicked && !late.double_clicked);
}

#[test]
fn tab_strip_selects_and_closes_tabs() {
    let assets = Assets::load(ThemeChoice::Dark).unwrap();
    let mut state = UiState::default();
    let area = rect(0, 0, 200, 14);
    let tabs = [
        Tab {
            label: "one".to_owned(),
            dirty: false,
            closable: false,
        },
        Tab {
            label: "two".to_owned(),
            dirty: true,
            closable: true,
        },
    ];
    let mut active = Some(0);
    let draw = |ui: &mut Ui<'_>, active: Option<usize>| {
        tab_strip(ui, Id::new("tabs"), area, &tabs, active)
    };
    let home_width = area.height + 4;
    let tab_width =
        |label: &str, dirty: bool| text_width(&assets.font, label) + i32::from(dirty) * 4 + 8 + 10;
    let first = rect(home_width, 0, tab_width("one", false), area.height);
    let second = rect(first.right(), 0, tab_width("two", true), area.height);
    assert_eq!(
        click(&assets, &mut state, (home_width / 2, 7), |ui| draw(
            ui, active
        )),
        TabAction::Home
    );
    active = None;
    assert_eq!(
        click(&assets, &mut state, (second.x + 3, 7), |ui| draw(
            ui, active
        )),
        TabAction::Select(1)
    );
    active = Some(1);
    assert_eq!(
        click(&assets, &mut state, (second.right() - 5, 7), |ui| draw(
            ui, active
        )),
        TabAction::Close(1)
    );
    let at = (first.x + 3, 7);
    frame(&assets, &mut state, &pointer(at, middle, true), |ui| {
        draw(ui, active)
    });
    assert_eq!(
        frame(&assets, &mut state, &pointer(at, middle, true), |ui| draw(
            ui, active
        )),
        TabAction::Close(0)
    );
    assert_eq!(
        frame(&assets, &mut state, &pointer(at, left, false), |ui| draw(
            ui, active
        )),
        TabAction::None
    );
}
