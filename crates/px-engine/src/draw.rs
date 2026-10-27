use px_core::{CanvasMut, Paint, snap_coord, snap_extent};
use px_expr::{Env, EvalError};
use px_raster::{Point, Rect, draw_line, fill_ellipse, fill_rect};

use crate::model::{Bounds, Geometry, Param, Segment};
use crate::render::RenderError;

type Recover<'r> = dyn FnMut(EvalError) -> Result<f64, RenderError> + 'r;

struct Values<'a, 'r> {
    env: &'a Env,
    recover: &'a mut Recover<'r>,
}

impl Values<'_, '_> {
    fn of(&mut self, param: &Param) -> Result<f64, RenderError> {
        param.value(self.env).or_else(|error| (self.recover)(error))
    }
}

impl Geometry {
    pub(crate) fn draw(
        &self,
        canvas: &mut CanvasMut<'_>,
        env: &Env,
        paint: Paint,
        recover: &mut Recover<'_>,
    ) -> Result<(), RenderError> {
        let mut values = Values { env, recover };
        match self {
            Self::Rect(bounds) => fill_rect(canvas, bounds.snapped(&mut values)?, paint),
            Self::Ellipse(bounds) => fill_ellipse(canvas, bounds.snapped(&mut values)?, paint),
            Self::Line(segment) => {
                let (from, to) = segment.snapped(&mut values)?;
                draw_line(canvas, from, to, paint);
            }
        }
        Ok(())
    }
}

impl Bounds {
    fn snapped(&self, values: &mut Values<'_, '_>) -> Result<Rect, RenderError> {
        Ok(Rect {
            x: snap_coord(values.of(&self.x)?),
            y: snap_coord(values.of(&self.y)?),
            width: snap_extent(values.of(&self.width)?),
            height: snap_extent(values.of(&self.height)?),
        })
    }
}

impl Segment {
    fn snapped(&self, values: &mut Values<'_, '_>) -> Result<(Point, Point), RenderError> {
        let from = Point {
            x: snap_coord(values.of(&self.x1)?),
            y: snap_coord(values.of(&self.y1)?),
        };
        let to = Point {
            x: snap_coord(values.of(&self.x2)?),
            y: snap_coord(values.of(&self.y2)?),
        };
        Ok((from, to))
    }
}
