use core::ops::Range;

use px_core::Paint;
use px_ui::{Area, Id, Rect, Ui, label, rect};

pub(super) const CELL: i32 = 8;
const NARROWEST: i32 = 4;
pub(super) const NAME: i32 = 44;
pub(super) const ROW_HEIGHT: i32 = CELL + 2;

#[derive(Clone, Copy, Debug)]
pub(super) struct Columns {
    pub(super) cells: Rect,
    pub(super) cell: i32,
    pub(super) first: u32,
    pub(super) count: u32,
}

impl Columns {
    pub(super) fn follow(ui: &mut Ui<'_>, id: Id, cells: Rect, frame: u32, frames: u32) -> Self {
        let wanted = cells.width / i32::try_from(frames).unwrap_or(i32::MAX).max(1);
        let cell = wanted.clamp(NARROWEST, CELL);
        let visible = u32::try_from((cells.width / cell).max(1))
            .unwrap_or(1)
            .min(frames);
        let latest = frames - visible;
        let kept = ui
            .state
            .scroll
            .get(&id)
            .and_then(|first| u32::try_from(*first).ok())
            .unwrap_or(0)
            .min(latest);
        let first = if frame < kept || frame >= kept + visible {
            frame.saturating_sub(visible / 2).min(latest)
        } else {
            kept
        };
        ui.state
            .scroll
            .insert(id, i32::try_from(first).unwrap_or(0));
        Self {
            cells,
            cell,
            first,
            count: visible,
        }
    }

    pub(super) fn frames(self) -> Range<u32> {
        self.first..self.first + self.count
    }

    pub(super) fn x_of(self, frame: u32) -> i32 {
        self.cells.x + i32::try_from(frame.saturating_sub(self.first)).unwrap_or(0) * self.cell
    }

    pub(super) fn frame_at(self, x: i32) -> u32 {
        let column = u32::try_from(((x - self.cells.x) / self.cell).max(0)).unwrap_or(0);
        self.first + column.min(self.count - 1)
    }

    pub(super) fn center(self, frame: u32) -> i32 {
        self.x_of(frame) + self.cell / 2
    }

    pub(super) fn right(self) -> i32 {
        self.x_of(self.first + self.count).min(self.cells.right())
    }
}

pub(super) fn name_cell(
    ui: &mut Ui<'_>,
    (grid, y): (Rect, i32),
    text: &str,
    selected: bool,
) -> Rect {
    let roles = ui.theme.roles;
    let area = rect(grid.x, y, NAME, CELL + 1);
    if selected {
        ui.canvas.fill(area, roles.accent);
    }
    let ink: Paint = if selected { roles.ink } else { roles.ink_dim };
    label(ui, area, &format!(" {text}"), ink);
    area
}

pub(super) fn visible((grid, y): (Rect, i32)) -> bool {
    y + CELL >= grid.y && y <= grid.bottom()
}
