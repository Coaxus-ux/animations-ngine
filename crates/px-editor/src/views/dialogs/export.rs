use std::path::PathBuf;

use px_export::{Padding, SheetLayout, place};
use px_ui::{
    Area, DialogAction, Id, ROW, Rect, Ui, button, checkbox, label, number_field, ok_cancel,
    segmented, text_field,
};

use super::Context;
use crate::app::{Dialog, Form, Intent, keys};
use crate::domain::Document;
use crate::domain::export::{
    ExportJob, ExportKind, FrameScope, KIND_LABELS, LAYOUT_LABELS, MAX_SCALE, frame_count,
    layout_from, layout_index,
};

pub(super) const TITLE: &str = "Export";
pub(super) const SIZE: (i32, i32) = (360, 211);

const FRAME_LABELS: [&str; 2] = ["All", "Current"];
const CURRENT: usize = 1;
const LABEL_WIDTH: i32 = 56;
const ROW_HEIGHT: i32 = ROW + 2;
const GAP: i32 = 2;
const BROWSE_WIDTH: i32 = 18;
const MAX_COUNT: i64 = 256;
const MAX_PADDING: i64 = 64;
const GIF: usize = ExportKind::Gif.index();
const PNG: usize = ExportKind::Png.index();
const SEQUENCE: usize = ExportKind::PngSequence.index();
const SHEET: usize = ExportKind::Sheet {
    layout: SheetLayout::HorizontalStrip,
    padding: Padding {
        border: 0,
        shape: 0,
    },
    json: true,
}
.index();
const GRID: usize = layout_index(SheetLayout::Rows { columns: 1 });

pub(super) fn draw(ui: &mut Ui<'_>, context: &mut Context<'_>, dialog: &mut Dialog, body: Rect) {
    let body = body.inset(4);
    let (buttons, fields) = body.take_bottom(ROW_HEIGHT);
    let mut rows = Rows { rest: fields };
    let form = &mut dialog.form;
    let kind_field = labeled(ui, rows.next(), "Kind");
    segmented(
        ui,
        id(keys::KIND),
        kind_field,
        &KIND_LABELS,
        form.choice(keys::KIND, 0),
    );
    let kind = *form.choice(keys::KIND, 0);
    let mut order = Vec::new();
    if kind == SHEET {
        sheet_rows(ui, &mut rows, form, &mut order);
    }
    let scale_field = labeled(ui, rows.next(), "Scale");
    number_field(
        ui,
        id(keys::SCALE),
        scale_field,
        form.number(keys::SCALE, 1),
        (1, i64::from(MAX_SCALE)),
    );
    order.push(id(keys::SCALE));
    toggle(
        ui,
        rows.next(),
        keys::TRANSPARENT,
        "Transparent background",
        true,
        form,
    );
    if kind != GIF {
        let row = rows.next();
        let (trim_field, split_field) = row.take_left(row.width / 2);
        toggle(ui, trim_field, keys::TRIM, "Trim sprite", false, form);
        if kind == SHEET || kind == SEQUENCE {
            toggle(
                ui,
                split_field,
                keys::SPLIT_LAYERS,
                "Split layers",
                false,
                form,
            );
        }
    }
    if kind != PNG {
        let frames_field = labeled(ui, rows.next(), "Frames");
        segmented(
            ui,
            id(keys::FRAMES),
            frames_field,
            &FRAME_LABELS,
            form.choice(keys::FRAMES, 0),
        );
    }
    let out_field = labeled(ui, rows.next(), "Output");
    let (browse, out_field) = out_field.take_right(BROWSE_WIDTH);
    text_field(ui, id(keys::OUT), out_field, form.text(keys::OUT, ""));
    order.push(id(keys::OUT));
    let browse_clicked = button(ui, id("browse"), browse, "...");

    let job = job_from(form, context.view.frame);
    let ink = ui.theme.roles.ink_dim;
    label(ui, rows.next(), &preview(context.document, &job), ink);
    if browse_clicked {
        context.intents.push(Intent::Export(ExportJob {
            out: PathBuf::new(),
            ..job
        }));
        return;
    }
    match ok_cancel(ui, id("ok"), buttons, "Export", true) {
        DialogAction::Ok => {
            let picks_path = job.out.as_os_str().is_empty();
            context.intents.push(Intent::Export(job));
            if !picks_path {
                context.intents.push(Intent::CloseDialog);
            }
        }
        DialogAction::Cancel => context.intents.push(Intent::CloseDialog),
        DialogAction::None => {}
    }
}

fn sheet_rows(ui: &mut Ui<'_>, rows: &mut Rows, form: &mut Form, order: &mut Vec<Id>) {
    let layout_field = labeled(ui, rows.next(), "Layout");
    segmented(
        ui,
        id(keys::LAYOUT),
        layout_field,
        &LAYOUT_LABELS,
        form.choice(keys::LAYOUT, 0),
    );
    let layout = *form.choice(keys::LAYOUT, 0);
    if layout >= GRID {
        let text = if layout == GRID { "Columns" } else { "Rows" };
        let count_field = labeled(ui, rows.next(), text);
        number_field(
            ui,
            id(keys::COUNT),
            count_field,
            form.number(keys::COUNT, 1),
            (1, MAX_COUNT),
        );
        order.push(id(keys::COUNT));
    }
    let padding_field = labeled(ui, rows.next(), "Border");
    let (border_field, rest) = padding_field.take_left(padding_field.width / 2 - GAP);
    number_field(
        ui,
        id(keys::BORDER),
        border_field,
        form.number(keys::BORDER, 0),
        (0, MAX_PADDING),
    );
    let shape_field = labeled(ui, rest, "Shape");
    number_field(
        ui,
        id(keys::SHAPE),
        shape_field,
        form.number(keys::SHAPE, 0),
        (0, MAX_PADDING),
    );
    order.extend([id(keys::BORDER), id(keys::SHAPE)]);
    toggle(ui, rows.next(), keys::JSON, "JSON data", true, form);
}

struct Rows {
    rest: Rect,
}

impl Rows {
    fn next(&mut self) -> Rect {
        let (row, rest) = self.rest.take_top(ROW_HEIGHT);
        self.rest = rest.take_top(GAP).1;
        row
    }
}

fn id(name: &str) -> Id {
    Id::new("export").with_name(name)
}

fn labeled(ui: &mut Ui<'_>, row: Rect, text: &str) -> Rect {
    let (name, field) = row.take_left(LABEL_WIDTH);
    let ink = ui.theme.roles.ink;
    label(ui, name, text, ink);
    field
}

fn toggle(ui: &mut Ui<'_>, row: Rect, key: &str, text: &str, default: bool, form: &mut Form) {
    let flag = form.flag(key, default);
    if checkbox(ui, id(key), row.take_left(LABEL_WIDTH).1, text, *flag) {
        *flag = !*flag;
    }
}

fn job_from(form: &mut Form, current: u32) -> ExportJob {
    let small = |value: i64| u16::try_from(value).unwrap_or(0);
    let layout = layout_from(
        *form.choice(keys::LAYOUT, 0),
        small(*form.number(keys::COUNT, 1)),
    );
    let padding = Padding {
        border: small(*form.number(keys::BORDER, 0)),
        shape: small(*form.number(keys::SHAPE, 0)),
    };
    let kind = ExportKind::from_index(
        *form.choice(keys::KIND, 0),
        layout,
        padding,
        *form.flag(keys::JSON, true),
    );
    let current_only = kind == ExportKind::Png || *form.choice(keys::FRAMES, 0) == CURRENT;
    ExportJob {
        kind,
        scale: u8::try_from(*form.number(keys::SCALE, 1)).unwrap_or(1),
        transparent: *form.flag(keys::TRANSPARENT, true),
        trim: kind != ExportKind::Gif && *form.flag(keys::TRIM, false),
        split_layers: kind.splits() && *form.flag(keys::SPLIT_LAYERS, false),
        frames: if current_only {
            FrameScope::Current(current)
        } else {
            FrameScope::All
        },
        out: PathBuf::from(form.text(keys::OUT, "").as_str()),
    }
}

fn preview(document: Option<&Document>, job: &ExportJob) -> String {
    let Some(scene) = document.and_then(Document::scene) else {
        return "No scene to export".to_owned();
    };
    let size = scene.size();
    let count = frame_count(job.frames, scene.timing().duration.get());
    let scale = u32::from(job.scale);
    let scaled = |(width, height): (u32, u32)| format!("{}x{} px", width * scale, height * scale);
    let frame = (u32::from(size.width()), u32::from(size.height()));
    let per_layer = if job.split_layers && job.kind.splits() {
        " per layer"
    } else {
        ""
    };
    match job.kind {
        ExportKind::Sheet {
            layout, padding, ..
        } => place(count, (size.width(), size.height()), layout, padding).map_or_else(
            |error| error.to_string(),
            |(sheet, _)| format!("Sheet: {}{per_layer}", scaled(sheet)),
        ),
        ExportKind::Png => format!("1 frame of {}", scaled(frame)),
        ExportKind::Gif | ExportKind::PngSequence => {
            format!("{count} frames of {}{per_layer}", scaled(frame))
        }
    }
}
