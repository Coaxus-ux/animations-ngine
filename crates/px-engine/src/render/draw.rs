use super::RenderError;
use super::composite::{Evaluated, composite};
use super::frame::{Pass, object_seed};
use super::geometry::rasterize;
use super::mask::Mask;
use super::stamp::{Capture, Key, MAX_NESTED_CLIPS, Write, shape_depth};
use super::surface::{Region, Surface};
use super::values::Values;
use crate::model::{Geometry, MAX_CLIPS, Scene, Shape};

/// What every shape of a frame shares.
pub(crate) struct Context<'c> {
    pub(crate) clips: &'c [Region; MAX_CLIPS],
    pub(crate) light: [f64; 3],
}

/// Draws every shape in order, copying a shape's stamp when it lands where it was captured;
/// with `capture`, records new stamps.
pub(crate) fn draw_shapes(
    pass: &mut Pass<'_, '_>,
    context: &Context<'_>,
    mask: &mut Mask<'_>,
    surface: &mut Surface<'_>,
    mut capture: Option<&mut Capture>,
) -> Result<(), RenderError> {
    let scene = pass.scene;
    for (index, shape) in scene.shapes().iter().enumerate() {
        let style = &shape.style;
        let seed = style
            .seed
            .unwrap_or_else(|| object_seed(scene.seed(), index));
        let anchor = style.anchor.as_ref();
        let Some(values) = pass.values(index, seed, style.instance, style.layer, anchor)? else {
            if let Some(capture) = capture.as_deref_mut() {
                capture.skip();
            }
            continue;
        };
        let key = Key::new(values.transform, shape_clips(shape, context.clips));
        if scene.stamps().replay(index, &key, surface, values.report) {
            continue;
        }
        let before = values.report.non_finite_values;
        let mut capture = capture.as_deref_mut().filter(|_| can_stamp(shape));
        let writes = capture.as_deref_mut().map(Capture::writes);
        let shape_draw = ShapeDraw {
            scene,
            index,
            shape,
            seed,
            clips: &key.clips,
            light: context.light,
        };
        let (timed, non_finite) = shape_draw.draw(values, mask, surface, writes, before)?;
        if let Some(capture) = capture {
            capture.finish(index, (!timed).then_some(key), non_finite);
        }
    }
    Ok(())
}

/// The regions of the clips `shape` is drawn inside, in order, padded with defaults.
fn shape_clips(shape: &Shape, clips: &[Region; MAX_CLIPS]) -> [Region; MAX_NESTED_CLIPS] {
    let mut regions = [Region::default(); MAX_NESTED_CLIPS];
    for (target, clip) in regions.iter_mut().zip(&shape.style.clips) {
        *target = clips.get(*clip).copied().unwrap_or_default();
    }
    regions
}

/// Shapes whose drawing reads the surface (shades darken it, contact shadows too) or the
/// clock directly (particles) are drawn every frame.
fn can_stamp(shape: &Shape) -> bool {
    !matches!(
        shape.geometry,
        Geometry::Shade { .. } | Geometry::Particles(_)
    )
}

struct ShapeDraw<'d> {
    scene: &'d Scene,
    index: usize,
    shape: &'d Shape,
    seed: u64,
    clips: &'d [Region; MAX_NESTED_CLIPS],
    light: [f64; 3],
}

impl ShapeDraw<'_> {
    /// Draws the shape; returns whether anything about it changes over time or reads the
    /// surface (which rules out a stamp), and how many non-finite values it reported since
    /// `before`.
    fn draw(
        &self,
        mut values: Values<'_>,
        mask: &mut Mask<'_>,
        surface: &mut Surface<'_>,
        writes: Option<&mut Vec<Write>>,
        before: u32,
    ) -> Result<(bool, u32), RenderError> {
        let shape = self.shape;
        let style = &shape.style;
        if values.of(&style.visible)? <= 0.5 {
            return Ok((values.timed, values.report.non_finite_values - before));
        }
        let layer = self.scene.environment().layers.get(style.layer);
        let fog = match layer {
            Some(layer) => values.of(&layer.fog)?,
            None => 0.0,
        };
        let shade_steps = match &shape.geometry {
            Geometry::Shade { steps, .. } => values.of(steps)?,
            _ => 0.0,
        };
        let used = style.clips.len().min(MAX_NESTED_CLIPS);
        let evaluated = Evaluated {
            depth: shape_depth(self.index),
            seed: self.seed,
            opacity: values.of(&style.opacity)?,
            tone_shift: values.of(&style.tone_shift)?,
            contact_shadow: values.of(&style.contact_shadow)?,
            shade_steps,
            fog,
            fog_paint: layer.and_then(|layer| layer.fog_paint),
            clips: &self.clips[..used],
            light: self.light,
            transform: values.transform,
        };
        mask.clear();
        rasterize(&shape.geometry, &mut values, mask)?;
        composite(
            shape,
            &evaluated,
            self.scene.palette(),
            mask,
            surface,
            writes,
        );
        let timed = values.timed || evaluated.contact_shadow > 0.0;
        Ok((timed, values.report.non_finite_values - before))
    }
}
