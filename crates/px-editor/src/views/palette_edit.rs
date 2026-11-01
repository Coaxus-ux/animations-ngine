mod color;
mod ramps;

use std::path::PathBuf;

use px_core::Rgb;
use px_ui::{Area, Id, ROW, Rect, Ui, button, centered_label, label, rect, scroll_area};

use super::Context;
use crate::app::Intent;
use crate::assets::palettes;
use crate::domain::Document;
use crate::domain::palette_model::PaletteModel;
use crate::domain::palette_model::palette_path;

const BUTTON_ROW: i32 = ROW + 2;
const GAP: i32 = 3;

pub(super) fn palette_edit(ui: &mut Ui<'_>, context: &mut Context<'_>, area: Rect) {
    let roles = ui.theme.roles;
    let (top, rest) = area.take_top(BUTTON_ROW);
    let (done, caption) = top.take_right(40);
    label(ui, caption.inset(1), "Palette edit", roles.ink_dim);
    if button(ui, id().with_name("done"), done.inset(1), "Done") {
        context.intents.push(Intent::TogglePaletteEdit);
    }
    if ui.hovering(done) {
        ui.tooltip("Back to painting with the palette (F4)");
    }
    let Some(document) = context.document else {
        return;
    };
    let model = match palette_path(document).and_then(|path| {
        PaletteModel::from_source(document.source(), &path)
            .ok_or_else(|| "The palette node could not be read".to_owned())
    }) {
        Ok(model) => model,
        Err(status) => {
            centered_label(ui, rest, &status, roles.ink_dim);
            return;
        }
    };
    let selected = selection(context, &model);
    let height =
        ramps::height(&model) + BUTTON_ROW + selected.map_or(0, |_| color::HEIGHT) + tools_height();
    let (view, offset) = scroll_area(ui, id().with_name("scroll"), rest, height);
    let clip = ui.canvas.clip_to(view);
    let mut column = Column {
        x: view.x,
        y: view.y - offset,
        width: view.width,
        view,
    };
    ramps::rows(ui, context, &model, selected, &mut column);
    let add = column.row(BUTTON_ROW);
    if column.shows(add) && button(ui, id().with_name("add"), add.inset(1), "Add ramp") {
        context.intents.push(Intent::AddRamp {
            name: "ramp".to_owned(),
            colors: Vec::new(),
        });
    }
    if let Some((ramp, tone)) = selected {
        color::editor(ui, context, &model, (ramp, tone), &mut column);
    }
    tools(ui, context, &mut column);
    ui.canvas.restore_clip(clip);
}

pub(super) struct Column {
    x: i32,
    y: i32,
    width: i32,
    view: Rect,
}

impl Column {
    pub(super) fn row(&mut self, height: i32) -> Rect {
        let row = rect(self.x, self.y, self.width, height);
        self.y += height;
        row
    }

    pub(super) fn shows(&self, row: Rect) -> bool {
        row.bottom() > self.view.y && row.y < self.view.bottom()
    }
}

pub(super) fn id() -> Id {
    Id::new("palette-edit")
}

fn selection(context: &Context<'_>, model: &PaletteModel) -> Option<(usize, usize)> {
    let text = context.fields.get(&id().with_name("selected"))?;
    let (ramp, tone) = text.split_once('/')?;
    let tone: usize = tone.parse().ok()?;
    let index = model.ramps.iter().position(|model| model.name == ramp)?;
    (tone < model.ramps[index].colors.len()).then_some((index, tone))
}

pub(super) fn selected_ramp(context: &Context<'_>, document: &Document) -> Option<(String, Rgb)> {
    let picked = palette_path(document).ok().and_then(|path| {
        let model = PaletteModel::from_source(document.source(), &path)?;
        let (ramp, tone) = selection(context, &model)?;
        let ramp = &model.ramps[ramp];
        Some((ramp.name.clone(), *ramp.colors.get(tone)?))
    });
    picked.or_else(|| {
        let paint = context.editing.primary?;
        let loaded = document.loaded()?;
        let name = loaded.paint_names.ramp(paint.ramp)?.to_owned();
        let color = loaded.scene.palette().resolve(paint).ok()?;
        Some((name, color))
    })
}

pub(super) fn select(context: &mut Context<'_>, ramp: &str, tone: usize) {
    context
        .fields
        .insert(id().with_name("selected"), format!("{ramp}/{tone}"));
}

fn tools_height() -> i32 {
    let presets = i32::try_from(palettes::names().count()).unwrap_or(0);
    GAP + BUTTON_ROW + BUTTON_ROW + (presets + 1) / 2 * BUTTON_ROW
}

fn tools(ui: &mut Ui<'_>, context: &mut Context<'_>, column: &mut Column) {
    column.row(GAP);
    let file_buttons: [(&str, Intent, &str); 2] = [
        (
            "Import .hex/.gpl",
            Intent::ImportPalette(PathBuf::new()),
            "Add the colors of a Lospec .hex or GIMP .gpl file as ramps",
        ),
        (
            "Save palette as",
            Intent::SavePalette(PathBuf::new()),
            "Write the palette to its own KDL file, to `use` from other scenes",
        ),
    ];
    for (text, intent, tip) in file_buttons {
        let row = column.row(BUTTON_ROW);
        if !column.shows(row) {
            continue;
        }
        if button(ui, id().with_name(text), row.inset(1), text) {
            context.intents.push(intent);
        }
        if ui.hovering(row) {
            ui.tooltip(tip);
        }
    }
    let names: Vec<&str> = palettes::names().collect();
    for pair in names.chunks(2) {
        let row = column.row(BUTTON_ROW);
        if !column.shows(row) {
            continue;
        }
        let (left, right) = row.take_left(row.width / 2);
        for (name, cell) in pair.iter().zip([left, right]) {
            if button(ui, id().with_name(name), cell.inset(1), name) {
                context
                    .intents
                    .push(Intent::ApplyPreset((*name).to_owned()));
            }
            if ui.hovering(cell) {
                ui.tooltip("Replace every ramp with this preset's");
            }
        }
    }
}
