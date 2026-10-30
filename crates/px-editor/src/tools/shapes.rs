use px_raster::{Point, Rect, draw_line, fill_ellipse, fill_rect};

use super::{Effect, Tool, Work};
use crate::domain::Selection;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Line,
    Rect,
    Ellipse,
}

/// Line, rectangle and ellipse: redrawn from the stroke's start to the pointer on every
/// move, over the cel as it was when the stroke began.
pub struct ShapeTool {
    kind: Kind,
    start: (i32, i32),
}

impl ShapeTool {
    pub const fn new(kind: Kind) -> Self {
        Self {
            kind,
            start: (0, 0),
        }
    }

    fn draw(&self, work: &mut Work<'_>, at: (i32, i32)) -> Effect {
        *work.raster = work.base.clone();
        let end = if work.constrain {
            constrained(self.kind, self.start, at)
        } else {
            at
        };
        let (x0, x1) = (self.start.0.min(end.0), self.start.0.max(end.0));
        let (y0, y1) = (self.start.1.min(end.1), self.start.1.max(end.1));
        let bounds = Rect {
            x: x0,
            y: y0,
            width: x1 - x0 + 1,
            height: y1 - y0 + 1,
        };
        let (width, height) = (work.raster.width(), work.raster.height());
        // The shape goes into a mask first, so outlines can be cut from it.
        let mut shape = Selection::none(width, height);
        match self.kind {
            Kind::Line => {
                let point = |(x, y): (i32, i32)| Point { x, y };
                draw_line(&mut shape, point(self.start), point(end));
            }
            Kind::Rect => fill_rect(&mut shape, bounds),
            Kind::Ellipse => fill_ellipse(&mut shape, bounds),
        }
        let outline = self.kind != Kind::Line && !work.filled;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let painted = if outline {
                    shape.is_edge(x, y)
                } else {
                    shape.contains(x, y)
                };
                if painted && work.selection.allows(x, y) {
                    work.raster.set(x, y, work.paint);
                }
            }
        }
        Effect {
            repaint: true,
            ..Effect::default()
        }
    }
}

/// Lines snap to the nearest 45°; boxes become squares.
fn constrained(kind: Kind, start: (i32, i32), at: (i32, i32)) -> (i32, i32) {
    let (dx, dy) = (at.0 - start.0, at.1 - start.1);
    let side = dx.abs().max(dy.abs());
    match kind {
        Kind::Line if dx.abs() > 2 * dy.abs() => (at.0, start.1),
        Kind::Line if dy.abs() > 2 * dx.abs() => (start.0, at.1),
        _ => (start.0 + side * dx.signum(), start.1 + side * dy.signum()),
    }
}

impl Tool for ShapeTool {
    fn press(&mut self, work: &mut Work<'_>, at: (i32, i32)) -> Effect {
        self.start = at;
        self.draw(work, at)
    }

    fn drag(&mut self, work: &mut Work<'_>, at: (i32, i32)) -> Effect {
        self.draw(work, at)
    }

    fn release(&mut self, work: &mut Work<'_>, at: (i32, i32)) -> Effect {
        Effect {
            commit: true,
            ..self.draw(work, at)
        }
    }
}
