use px_ui::{Area, Id, Rect, Ui};

use super::panel_frame::{COLLAPSED, is_collapsed, panel_view};
use super::{Context, tools};
use crate::app::{PanelKind, Side};

pub(super) const GRIP: i32 = 3;

pub(super) fn column(ui: &mut Ui<'_>, context: &mut Context<'_>, kinds: &[PanelKind], area: Rect) {
    let extra = |kind: PanelKind| {
        if kind == PanelKind::Palette {
            tools::PAINTS
        } else {
            0
        }
    };
    let folded: Vec<bool> = kinds
        .iter()
        .map(|kind| is_collapsed(context, *kind))
        .collect();
    let height_of = |index: usize| {
        let kind = kinds[index];
        extra(kind) + if folded[index] { COLLAPSED } else { 0 }
    };
    let open = folded.iter().filter(|folded| !**folded).count();
    let fixed: i32 = (0..kinds.len()).map(height_of).sum();
    let share = (area.height - fixed) / i32::try_from(open.max(1)).unwrap_or(1);
    let last_open = folded.iter().rposition(|folded| !*folded);
    let mut rest = area;
    for (index, kind) in kinds.iter().enumerate() {
        let height = if Some(index) == last_open {
            rest.height
                - (index + 1..kinds.len())
                    .filter(|later| folded[*later])
                    .map(height_of)
                    .sum::<i32>()
        } else if folded[index] {
            height_of(index)
        } else {
            share + extra(*kind)
        };
        let (slot, remaining) = rest.take_top(height);
        rest = remaining;
        place(ui, context, *kind, slot);
    }
    ui.canvas.fill(rest, ui.theme.roles.panel);
}

fn place(ui: &mut Ui<'_>, context: &mut Context<'_>, kind: PanelKind, slot: Rect) {
    if kind == PanelKind::Palette {
        let (paints, palette) = slot.take_bottom(tools::PAINTS);
        panel_view(ui, context, kind, palette);
        tools::paints(ui, context, paints);
    } else {
        panel_view(ui, context, kind, slot);
    }
}

pub(super) fn grip(
    ui: &mut Ui<'_>,
    context: &mut Context<'_>,
    side: Side,
    (column, grip): (Rect, Rect),
) {
    let id = Id::new("column-grip").with(side as u64);
    let response = ui.interact(id, grip);
    if response.held {
        let x = ui.input.pointer_anywhere.0;
        let width = match side {
            Side::Left => x - GRIP / 2 - column.x,
            Side::Right => column.right() - x - GRIP + GRIP / 2,
        };
        context.panels.set_width(side, width);
    }
    let roles = ui.theme.roles;
    let ink = if response.held {
        roles.accent
    } else if response.hovered {
        roles.panel_light
    } else {
        roles.panel_dark
    };
    ui.canvas.fill(grip, ink);
}
