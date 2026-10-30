//! Menus and popups with the editor's own theme: an open menu takes the click, and the
//! widget under it doesn't get it too.
#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use px_editor::assets::{Assets, ThemeChoice};
use px_ui::{Buttons, Id, Input, Menu, MenuItem, Ui, UiCanvas, UiState, button, menu_bar, rect};

/// One frame with a "Tools" menu at the top and a button under where it drops down;
/// returns (chosen menu item, whether the button was clicked).
fn frame(assets: &Assets, state: &mut UiState, input: &Input) -> (Option<(usize, usize)>, bool) {
    let mut canvas = UiCanvas::new(200, 120, assets.theme.roles.window);
    let mut ui = Ui::new(
        &mut canvas,
        &assets.theme,
        &assets.font,
        &assets.icons,
        input,
        state,
    );
    let clicked = button(&mut ui, Id::new("under"), rect(0, 14, 120, 40), "Under");
    let menus = [Menu {
        title: "Tools",
        items: vec![
            MenuItem::new("First", ""),
            MenuItem::new("Second", "Ctrl+2"),
        ],
    }];
    let chosen = menu_bar(&mut ui, rect(0, 0, 200, 12), &menus);
    ui.finish();
    (chosen, clicked)
}

fn click(at: (i32, i32), pressed: bool) -> Input {
    Input {
        pointer: Some(at),
        pointer_anywhere: at,
        down: Buttons {
            left: pressed,
            ..Buttons::default()
        },
        pressed: Buttons {
            left: pressed,
            ..Buttons::default()
        },
        released: Buttons {
            left: !pressed,
            ..Buttons::default()
        },
        ..Input::default()
    }
}

#[test]
fn an_open_menu_takes_the_click_from_what_is_under_it() {
    let assets = Assets::load(ThemeChoice::Dark).unwrap();
    let mut state = UiState::default();
    frame(&assets, &mut state, &click((8, 5), true));
    assert_eq!(state.open_menu, Some(0));
    frame(&assets, &mut state, &click((8, 5), false));
    assert_eq!(state.open_menu, Some(0));
    let second = (20, 12 + 1 + 13 + 6);
    frame(&assets, &mut state, &click(second, true));
    let (chosen, under) = frame(&assets, &mut state, &click(second, false));
    assert_eq!(chosen, Some((0, 1)));
    assert!(!under, "the button under the menu got the click");
    assert_eq!(state.open_menu, None);
}

#[test]
fn clicking_away_closes_a_menu_without_choosing() {
    let assets = Assets::load(ThemeChoice::Dark).unwrap();
    let mut state = UiState::default();
    frame(&assets, &mut state, &click((8, 5), true));
    let (chosen, _) = frame(&assets, &mut state, &click((190, 110), true));
    assert_eq!(chosen, None);
    assert_eq!(state.open_menu, None);
}
