use px_core::{MAX_FPS, MAX_FRAMES};
use px_ui::{
    Area, DialogAction, Id, ROW, Rect, Ui, checkbox, focus_next, label, number_field, ok_cancel,
};

use super::Context;
use crate::app::{Dialog, Intent, keys};

pub(super) const TITLE: &str = "Properties";
pub(super) const SIZE: (i32, i32) = (240, 120);

const FIELD: i32 = ROW + 2;
const GAP: i32 = 3;
const CAPTION: i32 = 64;
const CAPTIONS: [&str; 5] = [
    " Name",
    " Background",
    " Frames / s",
    " Frames",
    " Playback",
];

/// The scene's name and background (read here; the code view changes them), its frame rate,
/// length and looping, and Apply. Canvas Size owns the width and height.
pub(super) fn draw(ui: &mut Ui<'_>, context: &mut Context<'_>, dialog: &mut Dialog, body: Rect) {
    let roles = ui.theme.roles;
    let id = Id::new("properties");
    let body = body.inset(4);
    let (buttons, mut rest) = body.take_bottom(FIELD);
    let mut row = || {
        let (row, remaining) = rest.take_top(FIELD);
        rest = remaining.take_top(GAP).1;
        row.take_left(CAPTION)
    };
    let fps_id = id.with_name(keys::FPS);
    let frames_id = id.with_name(keys::DURATION);
    let [name, background, fps, frames, playback] = CAPTIONS;

    let (caption, value) = row();
    label(ui, caption, name, roles.ink_dim);
    label(ui, value, dialog.form.text(keys::NAME, ""), roles.ink);

    let (caption, value) = row();
    label(ui, caption, background, roles.ink_dim);
    label(ui, value, dialog.form.text(keys::BACKGROUND, ""), roles.ink);

    let (caption, field) = row();
    label(ui, caption, fps, roles.ink_dim);
    number_field(
        ui,
        fps_id,
        field,
        dialog.form.number(keys::FPS, 12),
        (1, i64::from(MAX_FPS)),
    );

    let (caption, field) = row();
    label(ui, caption, frames, roles.ink_dim);
    number_field(
        ui,
        frames_id,
        field,
        dialog.form.number(keys::DURATION, 1),
        (1, i64::from(MAX_FRAMES)),
    );

    let (caption, field) = row();
    label(ui, caption, playback, roles.ink_dim);
    let looping = dialog.form.flag(keys::LOOP, true);
    if checkbox(ui, id.with_name(keys::LOOP), field, "Loop", *looping) {
        *looping = !*looping;
    }
    let looping = *looping;
    focus_next(ui, &[fps_id, frames_id]);

    match ok_cancel(ui, id, buttons, "Apply", true) {
        DialogAction::Ok => {
            context.intents.push(Intent::SetTiming {
                fps: u16::try_from(*dialog.form.number(keys::FPS, 12)).unwrap_or(1),
                duration: u32::try_from(*dialog.form.number(keys::DURATION, 1)).unwrap_or(1),
                looping,
            });
            context.intents.push(Intent::CloseDialog);
        }
        DialogAction::Cancel => context.intents.push(Intent::CloseDialog),
        DialogAction::None => {}
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests fail loudly")]

    use px_format::Font;
    use px_ui::text_width;

    use super::*;

    #[test]
    fn every_caption_fits_its_column() {
        let font = Font::builtin().unwrap();
        for caption in CAPTIONS {
            assert!(text_width(&font, caption) <= CAPTION, "{caption:?}");
        }
    }
}
