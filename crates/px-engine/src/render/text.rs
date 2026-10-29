use core::f64::consts::TAU;

use px_core::{hash_seed, snap_coord};
use px_raster::{Rect, fill_rect};

use super::RenderError;
use super::geometry::unit;
use super::mask::Mask;
use super::values::Values;
use crate::model::Text;

/// Fills one glyph pixel per cell. Each cell's edges are snapped on their own, so the
/// pixels of a glyph stay touching at any size and zoom.
pub(crate) fn text(
    spec: &Text,
    values: &mut Values<'_>,
    mask: &mut Mask<'_>,
) -> Result<(), RenderError> {
    let (x, y) = (values.of(&spec.x)?, values.of(&spec.y)?);
    let size = values.of(&spec.size)?;
    let reveal = values.of(&spec.reveal)?;
    let scatter = values.of(&spec.scatter)?;
    let transform = values.transform;
    for (index, cell) in (0u64..).zip(&spec.cells) {
        if f64::from(cell.glyph) >= reveal {
            continue;
        }
        let [dx, dy] = if scatter == 0.0 {
            [0.0, 0.0]
        } else {
            let random = |key: u64| unit(hash_seed(values.env.seed, &[index, key]));
            let (angle, reach) = (random(0) * TAU, scatter * (0.5 + random(1)));
            [libm::cos(angle) * reach, libm::sin(angle) * reach]
        };
        let pixel = size * f64::from(cell.scale);
        let left = x + f64::from(cell.x) * size + dx;
        let top = y + f64::from(cell.y) * size + dy;
        let (left, width) = transform.span_x(left, pixel);
        let (top, bottom) = (transform.y(top), transform.y(top + pixel));
        let (x0, x1) = (snap_coord(left), snap_coord(left + width));
        let (y0, y1) = (snap_coord(top), snap_coord(bottom));
        fill_rect(
            mask,
            Rect {
                x: x0,
                y: y0,
                width: x1 - x0,
                height: y1 - y0,
            },
        );
    }
    Ok(())
}
