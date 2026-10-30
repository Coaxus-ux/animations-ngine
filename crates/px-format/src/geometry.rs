use px_engine::{Bounds, Geometry, Param, Particles, PixelArt, Segment, Text};
use px_expr::{Blend, Program};

use crate::extras::Extras;
use crate::schema::Kind;

/// Hands out a draft's compiled properties in schema order.
pub(crate) struct Next(pub(crate) std::vec::IntoIter<Param>);

impl Next {
    pub(crate) fn param(&mut self) -> Param {
        self.0.next().unwrap_or_default()
    }

    pub(crate) fn bounds(&mut self) -> Bounds {
        Bounds {
            x: self.param(),
            y: self.param(),
            width: self.param(),
            height: self.param(),
        }
    }

    pub(crate) fn segment(&mut self) -> Segment {
        Segment {
            x1: self.param(),
            y1: self.param(),
            x2: self.param(),
            y2: self.param(),
        }
    }
}

pub(crate) fn geometry(kind: Kind, extras: Extras, next: &mut Next) -> Geometry {
    match (kind, extras) {
        (Kind::Ellipse, _) => Geometry::Ellipse(next.bounds()),
        (Kind::RoundedRect, _) => Geometry::RoundedRect {
            bounds: next.bounds(),
            radius: next.param(),
        },
        (Kind::Line, _) => Geometry::Line {
            segment: next.segment(),
            width: next.param(),
        },
        (Kind::Capsule, _) => Geometry::Line {
            segment: next.segment(),
            width: doubled(next.param()),
        },
        (_, Extras::Points(points)) => Geometry::Polygon {
            x: next.param(),
            y: next.param(),
            points,
        },
        (_, Extras::Circles(circles)) => Geometry::Blob {
            x: next.param(),
            y: next.param(),
            circles,
            smooth: next.param(),
        },
        (Kind::Rope, _) => Geometry::Rope {
            segment: next.segment(),
            sag: next.param(),
            sway: next.param(),
            width: next.param(),
        },
        (_, Extras::Particles { count, fade }) => Geometry::Particles(Box::new(Particles {
            area: next.bounds(),
            velocity_x: next.param(),
            velocity_y: next.param(),
            gravity: next.param(),
            life: next.param(),
            size: next.param(),
            wind: next.param(),
            streak: next.param(),
            count,
            fade,
        })),
        (_, Extras::Text(cells)) => Geometry::Text(Box::new(Text {
            x: next.param(),
            y: next.param(),
            size: next.param(),
            reveal: next.param(),
            scatter: next.param(),
            cells,
        })),
        (_, Extras::Pixels(cels)) => Geometry::Pixels(Box::new(PixelArt {
            x: next.param(),
            y: next.param(),
            cels,
        })),
        (_, Extras::Shade { round }) => Geometry::Shade {
            bounds: next.bounds(),
            steps: next.param(),
            round,
        },
        _ => Geometry::Rect(next.bounds()),
    }
}

fn doubled(radius: Param) -> Param {
    match radius {
        Param::Fixed(radius) => Param::Fixed(radius * 2.0),
        Param::Expr(program) => Param::Expr(Program::blend(
            &program,
            &Program::fixed(2.0),
            Blend::Multiply,
        )),
    }
}
