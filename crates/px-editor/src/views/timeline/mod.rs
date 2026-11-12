mod cels;
mod columns;
mod controls;
mod keys;
mod ruler;

use px_ui::{Area, Id, Rect, Ui, rect, scroll_area};

use self::columns::{Columns, NAME, ROW_HEIGHT};
use self::controls::{BUTTON, controls};
use super::{Context, unavailable};
use crate::domain::timeline::{key_rows, markers};

pub(super) fn timeline(ui: &mut Ui<'_>, context: &mut Context<'_>, area: Rect) {
    let Some((document, scene)) = context
        .document
        .and_then(|document| Some((document, document.scene()?)))
    else {
        unavailable(ui, context, area);
        return;
    };
    let timing = scene.timing();
    let frames = timing.duration.get().max(1);
    let frame = context.view.frame % frames;
    let (bar, rest) = area.take_top(BUTTON + 2);
    controls(ui, context, bar, frame, frames);
    let (ruler_area, grid) = rest.take_top(ROW_HEIGHT);
    let id = Id::new("timeline");
    let columns = Columns::follow(
        ui,
        id.with_name("first"),
        ruler_area.take_left(NAME).1,
        frame,
        frames,
    );
    ruler::ruler(ui, context, ruler_area, columns, (frame, timing.fps.get()));

    let marks = markers(document);
    let keys = key_rows(document);
    let cels = cels::rows(document);
    let count = usize::from(!marks.is_empty()) + keys.len() + cels.len();
    let height = i32::try_from(count).unwrap_or(0) * ROW_HEIGHT;
    let (grid, offset) = scroll_area(ui, id.with_name("rows"), grid, height);
    let clip = ui.canvas.clip_to(grid);
    let highlight = rect(columns.x_of(frame), grid.y, columns.cell, grid.height);
    ui.canvas.fill(highlight, ui.theme.roles.panel_light);
    let mut y = grid.y - offset;
    if !marks.is_empty() {
        ruler::markers(ui, context, (grid, y), columns, &marks);
        y += ROW_HEIGHT;
    }
    for row in keys.iter().enumerate() {
        keys::row(ui, context, (grid, y), columns, row);
        y += ROW_HEIGHT;
    }
    for row in cels {
        cels::row(ui, context, (grid, y), columns, (scene, row));
        y += ROW_HEIGHT;
    }
    ui.canvas.restore_clip(clip);
}
