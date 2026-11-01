use px_core::{MAX_CANVAS_SIDE, MAX_FPS, MAX_FRAMES};
use px_ui::{
    Area, DialogAction, HEADER, Id, ROW, Rect, Ui, label, number_field, ok_cancel, segmented,
    text_field,
};

use super::Context;
use crate::app::{Dialog, Form, Intent, keys};
use crate::assets::palettes;
use crate::domain::NewSpec;

pub(super) const TITLE: &str = "New";
pub(super) const SIZE: (i32, i32) = (300, ROWS * PITCH + HEADER + 2 * PAD);

const FIELD: i32 = ROW + 2;
const GAP: i32 = 3;
const PITCH: i32 = FIELD + GAP;
const PAD: i32 = 4;
const ROWS: i32 = 6;
const LABEL: i32 = 60;
const BACKGROUNDS: [&str; 2] = ["First tone", "Last tone"];

pub(super) fn draw(ui: &mut Ui<'_>, context: &mut Context<'_>, dialog: &mut Dialog, body: Rect) {
    let defaults = defaults(context);
    let id = Id::new("new-doc");
    let ids = [
        id.with_name(keys::NAME),
        id.with_name(keys::WIDTH),
        id.with_name(keys::HEIGHT),
        id.with_name(keys::FPS),
        id.with_name(keys::FRAMES),
    ];
    let form = &mut dialog.form;
    let mut rows = body.inset(PAD);
    let mut row = || {
        let (taken, rest) = rows.take_top(PITCH);
        rows = rest;
        taken.take_top(FIELD).0
    };

    let name = labelled(ui, row(), "Name");
    text_field(ui, ids[0], name, form.text(keys::NAME, &defaults.name));

    let (left, right) = halves(row());
    let width = labelled(ui, left, "Width");
    let side = (1, i64::from(MAX_CANVAS_SIDE));
    number_field(
        ui,
        ids[1],
        width,
        form.number(keys::WIDTH, i64::from(defaults.width)),
        side,
    );
    let height = labelled(ui, right, "Height");
    number_field(
        ui,
        ids[2],
        height,
        form.number(keys::HEIGHT, i64::from(defaults.height)),
        side,
    );

    let (left, right) = halves(row());
    let fps = labelled(ui, left, "FPS");
    number_field(
        ui,
        ids[3],
        fps,
        form.number(keys::FPS, i64::from(defaults.fps)),
        (1, i64::from(MAX_FPS)),
    );
    let frames = labelled(ui, right, "Frames");
    number_field(
        ui,
        ids[4],
        frames,
        form.number(keys::FRAMES, i64::from(defaults.frames)),
        (1, i64::from(MAX_FRAMES)),
    );

    let names: Vec<&str> = palettes::names().collect();
    let preset = labelled(ui, row(), "Palette");
    segmented(
        ui,
        id.with_name(keys::PRESET),
        preset,
        &names,
        form.choice(keys::PRESET, 0),
    );

    let background = labelled(ui, row(), "Background");
    segmented(
        ui,
        id.with_name(keys::BACKGROUND),
        background,
        &BACKGROUNDS,
        form.choice(keys::BACKGROUND, 0),
    );

    match ok_cancel(ui, id.with_name("buttons"), row(), "Create", true) {
        DialogAction::Ok => {
            context
                .intents
                .push(Intent::NewDocument(spec_of(form, &names)));
            context.intents.push(Intent::CloseDialog);
        }
        DialogAction::Cancel => context.intents.push(Intent::CloseDialog),
        DialogAction::None => {}
    }
}

fn defaults(context: &Context<'_>) -> NewSpec {
    let mut spec = NewSpec::default();
    if let Some(scene) = context.document.and_then(crate::domain::Document::scene) {
        spec.width = scene.size().width();
        spec.height = scene.size().height();
        spec.fps = scene.timing().fps.get();
        spec.frames = scene.timing().duration.get();
    }
    spec
}

fn spec_of(form: &mut Form, presets: &[&str]) -> NewSpec {
    let defaults = NewSpec::default();
    let number = |form: &mut Form, key: &str| *form.number(key, 0);
    let preset = presets
        .get(*form.choice(keys::PRESET, 0))
        .map_or_else(|| defaults.preset.clone(), |name| (*name).to_owned());
    let background = (*form.choice(keys::BACKGROUND, 0) == 1)
        .then(|| palettes::preset(&preset).and_then(last_tone))
        .flatten();
    NewSpec {
        name: form.text(keys::NAME, &defaults.name).clone(),
        width: u16::try_from(number(form, keys::WIDTH)).unwrap_or(u16::MAX),
        height: u16::try_from(number(form, keys::HEIGHT)).unwrap_or(u16::MAX),
        fps: u16::try_from(number(form, keys::FPS)).unwrap_or(u16::MAX),
        frames: u32::try_from(number(form, keys::FRAMES)).unwrap_or(u32::MAX),
        preset,
        background,
    }
}

fn last_tone(preset: &str) -> Option<String> {
    let line = preset
        .lines()
        .map(str::trim_start)
        .find_map(|line| line.strip_prefix("ramp "))?;
    let mut quoted = line.split('"').skip(1).step_by(2);
    let ramp = quoted.next()?;
    let colors = quoted.filter(|part| part.starts_with('#')).count();
    Some(format!("{ramp}.{}", colors.checked_sub(1)?))
}

fn labelled(ui: &mut Ui<'_>, area: Rect, text: &str) -> Rect {
    let (caption, rest) = area.take_left(LABEL);
    let ink = ui.theme.roles.ink_dim;
    label(ui, caption, text, ink);
    rest
}

fn halves(area: Rect) -> (Rect, Rect) {
    let (left, right) = area.take_left(area.width / 2);
    (left.take_left(left.width - GAP).0, right)
}
