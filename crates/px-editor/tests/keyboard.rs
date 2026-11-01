#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use px_editor::assets::{Assets, ThemeChoice};
use px_ui::{
    DialogAction, Id, Input, Key, MENU_KEY, Menu, MenuItem, Modifiers, Ui, UiCanvas, UiState,
    button, button_if, menu_bar, modal, number_field, ok_cancel, rect,
};

fn frame<T>(
    assets: &Assets,
    state: &mut UiState,
    input: &Input,
    draw: impl FnOnce(&mut Ui<'_>) -> T,
) -> T {
    let mut canvas = UiCanvas::new(200, 120, assets.theme.roles.window);
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

fn press(keys: &[Key]) -> Input {
    Input {
        keys: keys.to_vec(),
        ..Input::default()
    }
}

fn shift_tab() -> Input {
    Input {
        modifiers: Modifiers {
            shift: true,
            ..Modifiers::default()
        },
        ..press(&[Key::Tab])
    }
}

fn two_buttons(ui: &mut Ui<'_>, first_enabled: bool) -> (bool, bool) {
    let first = button_if(
        ui,
        Id::new("first"),
        rect(0, 0, 40, 12),
        "First",
        first_enabled,
    );
    let second = button(ui, Id::new("second"), rect(0, 20, 40, 12), "Second");
    (first, second)
}

#[test]
fn tab_moves_the_ring_and_enter_presses_the_focused_button() {
    let assets = Assets::load(ThemeChoice::Dark).unwrap();
    let mut state = UiState::default();
    let draw = |ui: &mut Ui<'_>| two_buttons(ui, true);
    frame(&assets, &mut state, &Input::default(), draw);
    frame(&assets, &mut state, &press(&[Key::Tab]), draw);
    assert_eq!(state.key_focus, Some(Id::new("first")));
    assert_eq!(state.focus, None, "a button does not take typing");
    frame(&assets, &mut state, &press(&[Key::Tab]), draw);
    assert_eq!(
        frame(&assets, &mut state, &press(&[Key::Enter]), draw),
        (false, true)
    );
    frame(&assets, &mut state, &shift_tab(), draw);
    assert_eq!(
        frame(&assets, &mut state, &press(&[Key::Space]), draw),
        (true, false)
    );
}

#[test]
fn a_disabled_button_never_fires_and_tab_skips_it() {
    let assets = Assets::load(ThemeChoice::Dark).unwrap();
    let mut state = UiState::default();
    let draw = |ui: &mut Ui<'_>| two_buttons(ui, false);
    frame(&assets, &mut state, &Input::default(), draw);
    frame(&assets, &mut state, &press(&[Key::Tab]), draw);
    assert_eq!(state.key_focus, Some(Id::new("second")));
}

#[test]
fn a_mouse_press_or_escape_drops_the_ring() {
    let assets = Assets::load(ThemeChoice::Dark).unwrap();
    let mut state = UiState::default();
    let draw = |ui: &mut Ui<'_>| two_buttons(ui, true);
    frame(&assets, &mut state, &Input::default(), draw);
    frame(&assets, &mut state, &press(&[Key::Tab]), draw);
    frame(&assets, &mut state, &press(&[Key::Escape]), draw);
    assert_eq!(state.key_focus, None);
}

#[test]
fn enter_on_a_focused_cancel_cancels_instead_of_confirming() {
    let assets = Assets::load(ThemeChoice::Dark).unwrap();
    let mut state = UiState::default();
    state.modal = true;
    let draw = |ui: &mut Ui<'_>| {
        modal(ui, Id::new("dialog"), "Dialog", (120, 60), |ui, _, body| {
            ok_cancel(ui, Id::new("buttons"), body, "OK", true)
        })
    };
    frame(&assets, &mut state, &Input::default(), draw);
    frame(&assets, &mut state, &shift_tab(), draw);
    assert_eq!(
        frame(&assets, &mut state, &press(&[Key::Enter]), draw),
        DialogAction::Cancel
    );
    assert_eq!(
        frame(&assets, &mut state, &press(&[Key::Escape]), draw),
        DialogAction::Cancel
    );
}

#[test]
fn arrows_step_a_number_field_reached_with_tab() {
    let assets = Assets::load(ThemeChoice::Dark).unwrap();
    let mut state = UiState::default();
    let mut value = 5;
    let mut draw = |ui: &mut Ui<'_>| {
        number_field(ui, Id::new("count"), rect(0, 0, 60, 12), &mut value, (0, 9));
    };
    frame(&assets, &mut state, &Input::default(), &mut draw);
    frame(&assets, &mut state, &press(&[Key::Tab]), &mut draw);
    frame(&assets, &mut state, &press(&[Key::Up]), &mut draw);
    frame(&assets, &mut state, &press(&[Key::Up]), &mut draw);
    frame(&assets, &mut state, &press(&[Key::Enter]), &mut draw);
    assert_eq!(value, 7);
}

#[test]
fn the_menu_key_opens_the_menus_and_enter_picks_the_highlighted_item() {
    let assets = Assets::load(ThemeChoice::Dark).unwrap();
    let mut state = UiState::default();
    let draw = |ui: &mut Ui<'_>| {
        let menus = [
            Menu {
                title: "File",
                items: vec![
                    MenuItem::new("Closed", "").enabled(false),
                    MenuItem::new("Open", ""),
                    MenuItem::separator(),
                    MenuItem::new("Quit", ""),
                ],
            },
            Menu {
                title: "Edit",
                items: vec![MenuItem::new("Undo", "")],
            },
        ];
        menu_bar(ui, rect(0, 0, 200, 12), &menus)
    };
    frame(&assets, &mut state, &press(&[MENU_KEY]), draw);
    assert_eq!((state.open_menu, state.menu_item), (Some(0), Some(1)));
    frame(&assets, &mut state, &press(&[Key::Down]), draw);
    assert_eq!(state.menu_item, Some(3), "separators are skipped");
    frame(&assets, &mut state, &press(&[Key::Down]), draw);
    assert_eq!(state.menu_item, Some(1), "disabled items are skipped");
    frame(&assets, &mut state, &press(&[Key::Right]), draw);
    assert_eq!(state.open_menu, Some(1));
    assert_eq!(
        frame(&assets, &mut state, &press(&[Key::Enter]), draw),
        Some((1, 0))
    );
    assert_eq!(state.open_menu, None);
}
