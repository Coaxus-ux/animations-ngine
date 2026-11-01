use super::{App, WindowId};
use crate::app::Status;
use crate::domain::{Raster, Selection};
use crate::ports::FileStore;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SelectionOp {
    Copy,
    Cut,
    Paste,
    Flip { horizontal: bool },
    Delete,
}

impl App {
    pub(super) fn selection_op(
        &mut self,
        window: WindowId,
        op: SelectionOp,
        store: &mut dyn FileStore,
    ) {
        let mut target = match self.paint_target(window) {
            Ok(target) => target,
            Err(reason) => {
                self.editing.status = Status::problem(reason);
                return;
            }
        };
        let (width, height) = (target.raster.width(), target.raster.height());
        let selection = if self.editing.selection.is_empty() {
            Selection::rectangle(width, height, (0, 0), (width - 1, height - 1))
        } else {
            self.editing.selection.clone()
        };
        let mut raster = target.raster.clone();
        let changed = match op {
            SelectionOp::Copy => {
                self.editing.clipboard = Some(selection.lift(&mut raster.clone()));
                self.editing.status = Status::info("Copied");
                false
            }
            SelectionOp::Cut => {
                self.editing.clipboard = Some(selection.lift(&mut raster));
                true
            }
            SelectionOp::Delete => {
                selection.lift(&mut raster);
                true
            }
            SelectionOp::Flip { horizontal } => {
                selection.flip(&mut raster, horizontal);
                true
            }
            SelectionOp::Paste => {
                let Some(clipboard) = self.editing.clipboard.clone() else {
                    self.editing.status = Status::info("Nothing to paste");
                    return;
                };
                self.editing.selection = paste(&mut raster, &clipboard);
                self.editing.status = Status::info("Pasted: move it with the move tool (V)");
                true
            }
        };
        if !changed {
            return;
        }
        target.raster = raster;
        let command = target.command(&target.raster);
        if let Some(Err(error)) = self
            .document_mut()
            .map(|document| document.apply(&command, store))
        {
            self.editing.status = Status::problem(error);
        }
    }
}

fn paste(raster: &mut Raster, clipboard: &Raster) -> Selection {
    let mut points = Vec::new();
    for y in 0..clipboard.height() {
        for x in 0..clipboard.width() {
            if let Some(paint) = clipboard.get(x, y) {
                raster.set(x, y, Some(paint));
                points.push((x, y));
            }
        }
    }
    Selection::of_pixels(raster.width(), raster.height(), points)
}
