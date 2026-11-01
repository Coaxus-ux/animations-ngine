use crate::area::{Area, Rect, rect};
use crate::input::Key;
use crate::text::text_width;
use crate::ui::{Id, Ui};
use crate::widgets::{ROW, button, button_if, panel};

const BUTTON_PAD: i32 = 12;
const BUTTON_GAP: i32 = 4;
const MIN_BUTTON: i32 = 40;

pub fn modal<T>(
    ui: &mut Ui<'_>,
    id: Id,
    title: &str,
    size: (i32, i32),
    draw: impl FnOnce(&mut Ui<'_>, Rect, Rect) -> T,
) -> T {
    let bounds = ui.canvas.bounds();
    let area = bounds.centered(size.0, size.1).intersect(bounds);
    ui.popup(area, |ui| {
        let roles = ui.theme.roles;
        ui.canvas.dither(bounds, roles.backdrop);
        let (header, body) = panel(ui, area, title);
        let result = draw(ui, header, body);
        if ui.interact(id, area).pressed {
            ui.state.focus = None;
        }
        result
    })
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DialogAction {
    #[default]
    None,
    Ok,
    Cancel,
}

pub fn ok_cancel(ui: &mut Ui<'_>, id: Id, area: Rect, ok: &str, enabled: bool) -> DialogAction {
    let height = area.height.min(ROW + 2);
    let row = rect(
        area.x,
        area.y + (area.height - height) / 2,
        area.width,
        height,
    );
    let (cancel_area, rest) = row.take_right(button_width(ui, "Cancel"));
    let (ok_area, _) = rest.take_right(button_width(ui, ok) + BUTTON_GAP);
    let ok_area = ok_area.take_left(ok_area.width - BUTTON_GAP).0;
    let confirmed = button_if(ui, id.with_name("ok"), ok_area, ok, enabled);
    let cancelled = button(ui, id.with_name("cancel"), cancel_area, "Cancel");
    if cancelled || ui.take_key(Key::Escape) {
        DialogAction::Cancel
    } else if confirmed || (enabled && ui.take_key(Key::Enter)) {
        DialogAction::Ok
    } else {
        DialogAction::None
    }
}

fn button_width(ui: &Ui<'_>, text: &str) -> i32 {
    (text_width(ui.font, text) + BUTTON_PAD).max(MIN_BUTTON)
}
