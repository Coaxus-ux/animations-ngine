use px_core::snap_coord;
use px_raster::Rect;

use super::RenderError;
use super::mask::Mask;
use super::values::Values;
use crate::model::PixelArt;

pub(crate) fn pixels(
    art: &PixelArt,
    values: &mut Values<'_>,
    mask: &mut Mask<'_>,
) -> Result<(), RenderError> {
    let (x, y) = (values.of(&art.x)?, values.of(&art.y)?);
    values.timed |= art.is_animated();
    let env = values.env;
    let frame = match env.loop_period {
        Some(period) if period > 0.0 => {
            let frames = libm::round(period * env.fps).max(1.0);
            env.frame.rem_euclid(frames)
        }
        _ => env.frame,
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "frames are whole, non-negative and far below u32::MAX"
    )]
    let frame = px_core::floor(frame).max(0.0) as u32;
    let Some(cel) = art.cel_at(frame) else {
        return Ok(());
    };
    let transform = values.transform;
    let columns = usize::from(cel.width.max(1));
    for (row, paints) in (0i32..).zip(cel.pixels.chunks_exact(columns)) {
        let top = y + f64::from(cel.y + row);
        let (y0, y1) = (
            snap_coord(transform.y(top)),
            snap_coord(transform.y(top + 1.0)),
        );
        for (column, paint) in (0i32..).zip(paints) {
            let Some(paint) = *paint else { continue };
            let (left, width) = transform.span_x(x + f64::from(cel.x + column), 1.0);
            let (x0, x1) = (snap_coord(left), snap_coord(left + width));
            let rect = Rect {
                x: x0,
                y: y0,
                width: x1 - x0,
                height: y1 - y0,
            };
            mask.fill_paint(rect, paint);
        }
    }
    Ok(())
}
