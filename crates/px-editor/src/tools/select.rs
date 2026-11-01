use super::{Effect, Tool, Work};
use crate::domain::{Raster, Selection};

#[derive(Default)]
pub struct BoxSelect {
    start: (i32, i32),
}

impl Tool for BoxSelect {
    fn press(&mut self, work: &mut Work<'_>, at: (i32, i32)) -> Effect {
        self.start = at;
        self.drag(work, at)
    }

    fn drag(&mut self, work: &mut Work<'_>, at: (i32, i32)) -> Effect {
        let (width, height) = (work.raster.width(), work.raster.height());
        Effect {
            selection: Some(Selection::rectangle(width, height, self.start, at)),
            ..Effect::default()
        }
    }

    fn release(&mut self, work: &mut Work<'_>, at: (i32, i32)) -> Effect {
        if at == self.start {
            let (width, height) = (work.raster.width(), work.raster.height());
            return Effect {
                selection: Some(Selection::none(width, height)),
                ..Effect::default()
            };
        }
        self.drag(work, at)
    }
}

#[derive(Default)]
pub struct Lasso {
    points: Vec<(i32, i32)>,
}

impl Tool for Lasso {
    fn press(&mut self, _work: &mut Work<'_>, at: (i32, i32)) -> Effect {
        self.points = vec![at];
        Effect::default()
    }

    fn drag(&mut self, _work: &mut Work<'_>, at: (i32, i32)) -> Effect {
        if self.points.last() != Some(&at) {
            self.points.push(at);
        }
        Effect::default()
    }

    fn release(&mut self, work: &mut Work<'_>, _at: (i32, i32)) -> Effect {
        let (width, height) = (work.raster.width(), work.raster.height());
        let points = core::mem::take(&mut self.points);
        let selection = if points.len() < 3 {
            Selection::none(width, height)
        } else {
            Selection::lasso(width, height, &points)
        };
        Effect {
            selection: Some(selection),
            ..Effect::default()
        }
    }

    fn guide(&self) -> &[(i32, i32)] {
        &self.points
    }
}

#[derive(Default)]
pub struct Move {
    start: (i32, i32),
    lifted: Option<Raster>,
    left: Option<Raster>,
}

impl Tool for Move {
    fn press(&mut self, work: &mut Work<'_>, at: (i32, i32)) -> Effect {
        self.start = at;
        let mut left = work.base.clone();
        let everything;
        let selection = if work.selection.is_empty() {
            everything = Selection::rectangle(
                left.width(),
                left.height(),
                (0, 0),
                (left.width() - 1, left.height() - 1),
            );
            &everything
        } else {
            work.selection
        };
        self.lifted = Some(selection.lift(&mut left));
        self.left = Some(left);
        Effect::default()
    }

    fn drag(&mut self, work: &mut Work<'_>, at: (i32, i32)) -> Effect {
        let (Some(lifted), Some(left)) = (&self.lifted, &self.left) else {
            return Effect::default();
        };
        let (dx, dy) = (at.0 - self.start.0, at.1 - self.start.1);
        *work.raster = left.clone();
        for y in 0..lifted.height() {
            for x in 0..lifted.width() {
                if let Some(paint) = lifted.get(x, y) {
                    work.raster.set(x + dx, y + dy, Some(paint));
                }
            }
        }
        Effect {
            repaint: true,
            ..Effect::default()
        }
    }

    fn release(&mut self, work: &mut Work<'_>, at: (i32, i32)) -> Effect {
        let effect = self.drag(work, at);
        self.lifted = None;
        self.left = None;
        let (dx, dy) = (at.0 - self.start.0, at.1 - self.start.1);
        Effect {
            commit: true,
            selection: (!work.selection.is_empty()).then(|| work.selection.moved(dx, dy)),
            ..effect
        }
    }
}

pub struct Pick;

impl Tool for Pick {
    fn press(&mut self, _work: &mut Work<'_>, at: (i32, i32)) -> Effect {
        Effect {
            picked_object: Some(at),
            ..Effect::default()
        }
    }

    fn drag(&mut self, _work: &mut Work<'_>, _at: (i32, i32)) -> Effect {
        Effect::default()
    }

    fn release(&mut self, _work: &mut Work<'_>, _at: (i32, i32)) -> Effect {
        Effect::default()
    }
}
