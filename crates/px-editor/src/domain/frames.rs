use std::collections::BTreeSet;
use std::fmt::Display;

use px_core::{Frame, MAX_RAMPS, Paint, RampId, Rgb, Tone};
use px_engine::{FrameBuffer, Scene};
use px_export::RgbaImage;

use super::Document;
use super::export::FrameScope;
use super::visibility::with_hidden_layers;

/// The key color never reaches the output: cells on its ramp become transparent (D-018).
const KEY: Rgb = Rgb::new(255, 0, 255);

/// Renders the document's frames as color images, hidden layers left out. With
/// `transparent`, the background is replaced by a key ramp before rendering and every cell
/// still carrying that ramp becomes `None` (D-018).
pub fn render_frames(
    document: &Document,
    hidden: &BTreeSet<String>,
    transparent: bool,
    scope: FrameScope,
) -> Result<Vec<RgbaImage>, String> {
    let scene = visible_scene(document, hidden)?;
    let (scene, key) = if transparent {
        let (scene, key) = keyed(&scene)?;
        (scene, Some(key))
    } else {
        (scene, None)
    };
    let size = scene.size();
    let mut buffer = FrameBuffer::new(&scene);
    frames_in(&scene, scope)
        .map(|frame| {
            let view = buffer.render(&scene, frame).map_err(message)?;
            let pixels = view
                .cells()
                .iter()
                .map(|cell| {
                    if Some(cell.ramp) == key {
                        return Ok(None);
                    }
                    scene.palette().resolve(*cell).map(Some).map_err(message)
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(RgbaImage {
                width: u32::from(size.width()),
                height: u32::from(size.height()),
                pixels,
            })
        })
        .collect()
}

/// The document's scene with the layers in `hidden` made invisible.
pub fn visible_scene(document: &Document, hidden: &BTreeSet<String>) -> Result<Scene, String> {
    let scene = document
        .scene()
        .ok_or_else(|| "The scene doesn't load".to_owned())?;
    Ok(with_hidden_layers(document, scene, hidden).unwrap_or_else(|| scene.clone()))
}

/// The frames `scope` selects, in order; a current frame past the end wraps around.
pub fn frames_in(scene: &Scene, scope: FrameScope) -> Box<dyn Iterator<Item = Frame>> {
    let duration = scene.timing().duration;
    match scope {
        FrameScope::All => Box::new(duration.frames()),
        FrameScope::Current(index) => Box::new(std::iter::once(Frame::new(index % duration.get()))),
    }
}

pub fn message(error: impl Display) -> String {
    error.to_string()
}

fn keyed(scene: &Scene) -> Result<(Scene, RampId), String> {
    let mut spec = scene.spec().clone();
    if spec.palette.ramps().len() >= MAX_RAMPS {
        return Err(format!(
            "A transparent export needs a free palette slot for its key color, but the palette already has {MAX_RAMPS} ramps: export opaque or remove a ramp"
        ));
    }
    let key = spec.palette.push_ramp(&[KEY]).map_err(message)?;
    spec.background = Paint {
        ramp: key,
        tone: Tone::new(0),
    };
    Ok((Scene::new(spec).map_err(message)?, key))
}
