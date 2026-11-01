use std::collections::BTreeSet;
use std::fmt::Display;

use px_core::{Frame, MAX_RAMPS, Paint, RampId, Rgb, Tone};
use px_engine::{FrameBuffer, Scene};
use px_export::RgbaImage;

use super::Document;
use super::export::FrameScope;
use super::visibility::{hidden_layer_indices, with_hidden_layer_indices};

const KEY: Rgb = Rgb::new(255, 0, 255);

pub fn render_frames(
    document: &Document,
    hidden: &BTreeSet<String>,
    transparent: bool,
    scope: FrameScope,
) -> Result<Vec<RgbaImage>, String> {
    render_scene_frames(&visible_scene(document, hidden)?, transparent, scope)
}

pub fn render_scene_frames(
    scene: &Scene,
    transparent: bool,
    scope: FrameScope,
) -> Result<Vec<RgbaImage>, String> {
    let (scene, key) = if transparent {
        let (scene, key) = keyed(scene)?;
        (scene, Some(key))
    } else {
        (scene.clone(), None)
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

pub fn visible_scene(document: &Document, hidden: &BTreeSet<String>) -> Result<Scene, String> {
    visible_scene_without(document, &hidden_layer_indices(document, hidden))
}

pub fn visible_scene_without(
    document: &Document,
    hidden: &BTreeSet<usize>,
) -> Result<Scene, String> {
    let scene = document
        .scene()
        .ok_or_else(|| "The scene doesn't load".to_owned())?;
    Ok(with_hidden_layer_indices(document, scene, hidden).unwrap_or_else(|| scene.clone()))
}

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

pub fn keyed(scene: &Scene) -> Result<(Scene, RampId), String> {
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
    let scene = Scene::new(spec)
        .and_then(|keyed| keyed.with_detail(scene.detail()))
        .map_err(message)?;
    Ok((scene, key))
}
