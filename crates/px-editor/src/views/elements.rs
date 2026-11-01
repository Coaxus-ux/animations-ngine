use px_ui::{Area, Id, ROW, Rect, Ui, list_row_icon, rect, scroll_area};

use super::Context;
use crate::app::{CodeMode, Intent};
use crate::domain::{ElementKind, tree};

/// UI pixels each level of nesting moves a row right.
const INDENT: i32 = 6;

/// Every node of the scene file as a tree, in file order. A click selects the node (the
/// code view and inspector narrow to it), a click on the selected row lets it go, and a
/// double click also opens the code view.
pub(super) fn elements(ui: &mut Ui<'_>, context: &mut Context<'_>, area: Rect) {
    let Some(document) = context.document else {
        return;
    };
    let rows = tree::elements(document.source());
    let id = Id::new("elements");
    let total_height = i32::try_from(rows.len()).unwrap_or(0) * ROW;
    let (view, offset) = scroll_area(ui, id, area, total_height);
    let clip = ui.canvas.clip_to(view);
    for (slot, element) in (0i32..).zip(&rows) {
        let row = rect(view.x, view.y + slot * ROW - offset, view.width, ROW);
        if row.bottom() < view.y || row.y > view.bottom() {
            continue;
        }
        let indent = i32::try_from(element.depth).unwrap_or(0) * INDENT;
        let (_, rest) = row.take_left(indent);
        let selected = context.selected == Some(&element.path);
        let row_id = id.with_name(&element.path.to_string());
        let response = list_row_icon(
            ui,
            row_id,
            rest,
            icon(element.kind),
            &element.label,
            selected,
        );
        if response.clicked {
            let double = response.double_clicked;
            let path = (double || !selected).then(|| element.path.clone());
            context.intents.push(Intent::SelectElement(path));
            if double {
                context.intents.push(Intent::SetCode(CodeMode::Split));
            }
        }
    }
    ui.canvas.restore_clip(clip);
}

/// Row icons come from the small (8x8) set: `list_row_icon` gives them a `SMALL_ICON` slot.
const fn icon(kind: ElementKind) -> &'static str {
    match kind {
        ElementKind::Palette => "swatch",
        ElementKind::Ramp => "cel",
        ElementKind::Scene => "scene",
        ElementKind::Layer => "eye",
        ElementKind::Pixels => "pixels",
        ElementKind::Shape => "shape",
        ElementKind::Use | ElementKind::UseScene => "file",
        ElementKind::Other => "node",
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests fail loudly")]

    use px_ui::SMALL_ICON;

    use super::*;
    use crate::assets::{Assets, ThemeChoice};

    const KINDS: [ElementKind; 9] = [
        ElementKind::Palette,
        ElementKind::Ramp,
        ElementKind::Scene,
        ElementKind::Layer,
        ElementKind::Pixels,
        ElementKind::Shape,
        ElementKind::Use,
        ElementKind::UseScene,
        ElementKind::Other,
    ];

    #[test]
    fn every_element_kind_has_a_small_icon() {
        let assets = Assets::load(ThemeChoice::Dark).unwrap();
        for kind in KINDS {
            let name = icon(kind);
            let bitmap = assets
                .icons
                .get(name)
                .unwrap_or_else(|| panic!("{kind:?}: no icon {name:?}"));
            assert!(
                bitmap.width() <= SMALL_ICON && bitmap.height() <= SMALL_ICON,
                "{kind:?}: {name:?} is {}x{}",
                bitmap.width(),
                bitmap.height()
            );
        }
    }
}
