use super::{Effect, Tool, Work, line_points};

/// Pencil and eraser: one-pixel lines between pointer positions, kept pixel-perfect by
/// undoing the corner pixel of every L a stroke makes (RENDERING §7.2).
#[derive(Default)]
pub struct Pencil {
    last: Option<(i32, i32)>,
    /// The pixels painted this stroke, in order.
    trail: Vec<(i32, i32)>,
}

impl Pencil {
    fn plot(&mut self, work: &mut Work<'_>, at: (i32, i32)) {
        if !work.selection.allows(at.0, at.1) || self.trail.last() == Some(&at) {
            return;
        }
        work.raster.set(at.0, at.1, work.paint);
        self.trail.push(at);
        if let [.., a, b, c] = self.trail[..] {
            let diagonal = (a.0 - c.0).abs() == 1 && (a.1 - c.1).abs() == 1;
            let corner = (a.0 == b.0 || a.1 == b.1) && (b.0 == c.0 || b.1 == c.1);
            if diagonal && corner {
                work.raster.set(b.0, b.1, work.base.get(b.0, b.1));
                self.trail.remove(self.trail.len() - 2);
            }
        }
    }
}

impl Tool for Pencil {
    fn press(&mut self, work: &mut Work<'_>, at: (i32, i32)) -> Effect {
        self.trail.clear();
        self.last = Some(at);
        self.plot(work, at);
        Effect {
            repaint: true,
            ..Effect::default()
        }
    }

    fn drag(&mut self, work: &mut Work<'_>, at: (i32, i32)) -> Effect {
        let from = self.last.unwrap_or(at);
        for point in line_points(from, at).into_iter().skip(1) {
            self.plot(work, point);
        }
        self.last = Some(at);
        Effect {
            repaint: true,
            ..Effect::default()
        }
    }

    fn release(&mut self, _work: &mut Work<'_>, _at: (i32, i32)) -> Effect {
        self.last = None;
        Effect {
            commit: true,
            ..Effect::default()
        }
    }
}

pub struct Fill;

impl Tool for Fill {
    fn press(&mut self, work: &mut Work<'_>, at: (i32, i32)) -> Effect {
        let selection = work.selection;
        work.raster
            .flood(at.0, at.1, work.paint, work.global, |x, y| {
                selection.allows(x, y)
            });
        Effect {
            repaint: true,
            ..Effect::default()
        }
    }

    fn drag(&mut self, _work: &mut Work<'_>, _at: (i32, i32)) -> Effect {
        Effect::default()
    }

    fn release(&mut self, _work: &mut Work<'_>, _at: (i32, i32)) -> Effect {
        Effect {
            commit: true,
            ..Effect::default()
        }
    }
}

pub struct Eyedropper;

impl Tool for Eyedropper {
    fn press(&mut self, work: &mut Work<'_>, at: (i32, i32)) -> Effect {
        self.drag(work, at)
    }

    fn drag(&mut self, work: &mut Work<'_>, at: (i32, i32)) -> Effect {
        Effect {
            picked_paint: work
                .raster
                .contains(at.0, at.1)
                .then(|| work.raster.get(at.0, at.1)),
            ..Effect::default()
        }
    }

    fn release(&mut self, _work: &mut Work<'_>, _at: (i32, i32)) -> Effect {
        Effect::default()
    }
}
