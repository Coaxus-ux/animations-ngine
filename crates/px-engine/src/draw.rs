use px_core::{CanvasMut, Paint, snap_coord, snap_extent};
use px_expr::{Env, EvalError};
use px_raster::{Point, Rect, draw_line, fill_ellipse, fill_rect};

use crate::model::{Bounds, Geometry, Segment};

impl Geometry {
    /// Evaluates the parameters at `env`, snaps them to pixels and rasterizes.
    pub(crate) fn draw(
        &self,
        canvas: &mut CanvasMut<'_>,
        env: &Env,
        paint: Paint,
    ) -> Result<(), EvalError> {
        match self {
            Self::Rect(bounds) => fill_rect(canvas, bounds.snapped(env)?, paint),
            Self::Ellipse(bounds) => fill_ellipse(canvas, bounds.snapped(env)?, paint),
            Self::Line(segment) => {
                let (from, to) = segment.snapped(env)?;
                draw_line(canvas, from, to, paint);
            }
        }
        Ok(())
    }
}

impl Bounds {
    fn snapped(&self, env: &Env) -> Result<Rect, EvalError> {
        Ok(Rect {
            x: snap_coord(self.x.value(env)?),
            y: snap_coord(self.y.value(env)?),
            width: snap_extent(self.width.value(env)?),
            height: snap_extent(self.height.value(env)?),
        })
    }
}

impl Segment {
    fn snapped(&self, env: &Env) -> Result<(Point, Point), EvalError> {
        let from = Point {
            x: snap_coord(self.x1.value(env)?),
            y: snap_coord(self.y1.value(env)?),
        };
        let to = Point {
            x: snap_coord(self.x2.value(env)?),
            y: snap_coord(self.y2.value(env)?),
        };
        Ok((from, to))
    }
}
