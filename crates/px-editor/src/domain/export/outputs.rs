use px_engine::{FrameBuffer, Scene};
use px_export::{
    FrameTag, GifWriter, Padding, RgbaImage, Scale, SheetLayout, SheetMeta, Trim, crop, trim_bounds,
};

use super::targets::Target;
use super::{ExportJob, ExportKind, FrameScope, Output, frame_tags, sequence_name, stem};
use crate::domain::Document;
use crate::domain::frames::{
    frames_in, keyed, message, render_scene_frames, visible_scene_without,
};

pub(super) fn encode(
    job: &ExportJob,
    document: &Document,
    target: &Target,
    scale: Scale,
) -> Result<Vec<Output>, String> {
    let scene = visible_scene_without(document, &target.hidden)?
        .with_detail(job.detail)
        .map_err(message)?;
    match job.kind {
        ExportKind::Gif => Ok(vec![Output {
            path: target.out.clone(),
            bytes: gif(&scene, scale, job.frames, job.transparent)?,
        }]),
        ExportKind::Png => {
            let (frames, _) = frames(job, &scene)?;
            let image = frames
                .first()
                .ok_or_else(|| "no frame to export".to_owned())?;
            Ok(vec![Output {
                path: target.out.clone(),
                bytes: png(image, scale)?,
            }])
        }
        ExportKind::PngSequence => {
            let (frames, _) = frames(job, &scene)?;
            let stem = stem(&target.out);
            frames
                .iter()
                .zip(frames_in(&scene, job.frames))
                .map(|(image, frame)| {
                    let index = usize::try_from(frame.index()).map_err(message)?;
                    Ok(Output {
                        path: target.out.with_file_name(sequence_name(&stem, index)),
                        bytes: png(image, scale)?,
                    })
                })
                .collect()
        }
        ExportKind::Sheet {
            layout,
            padding,
            json,
        } => sheet(job, document, &scene, target, scale, layout, padding, json),
    }
}

fn frames(job: &ExportJob, scene: &Scene) -> Result<(Vec<RgbaImage>, Option<Trim>), String> {
    let frames = render_scene_frames(scene, job.transparent, job.frames)?;
    let bounds = job.trim.then(|| trim_bounds(&frames)).flatten();
    let Some((bounds, first)) = bounds.zip(frames.first()) else {
        return Ok((frames, None));
    };
    let side = |value: u32| u16::try_from(value).map_err(message);
    let trim = Trim {
        offset: (bounds.x, bounds.y),
        source: (side(first.width)?, side(first.height)?),
    };
    let cropped = frames.iter().map(|image| crop(image, bounds)).collect();
    Ok((cropped, Some(trim)))
}

fn gif(
    scene: &Scene,
    scale: Scale,
    scope: FrameScope,
    transparent: bool,
) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let timing = scene.timing();
    let (scene, key) = if transparent {
        let (scene, key) = keyed(scene)?;
        (scene, Some(key))
    } else {
        (scene.clone(), None)
    };
    let mut gif = match key {
        Some(key) => GifWriter::with_key(
            &mut bytes,
            scene.size(),
            scene.palette(),
            key,
            scale,
            timing.fps,
        ),
        None => GifWriter::new(&mut bytes, scene.size(), scene.palette(), scale, timing.fps),
    }
    .map_err(message)?;
    let mut buffer = FrameBuffer::new(&scene);
    for frame in frames_in(&scene, scope) {
        let view = buffer.render(&scene, frame).map_err(message)?;
        gif.add_frame(&view).map_err(message)?;
    }
    drop(gif);
    Ok(bytes)
}

fn png(image: &RgbaImage, scale: Scale) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    px_export::write_png_rgba(&mut bytes, image, scale).map_err(message)?;
    Ok(bytes)
}

#[expect(
    clippy::too_many_arguments,
    reason = "the sheet kind's fields, unpacked by the caller's match"
)]
fn sheet(
    job: &ExportJob,
    document: &Document,
    scene: &Scene,
    target: &Target,
    scale: Scale,
    layout: SheetLayout,
    padding: Padding,
    json: bool,
) -> Result<Vec<Output>, String> {
    let (frames, trim) = frames(job, scene)?;
    let (image, cells) = px_export::compose(&frames, layout, padding).map_err(message)?;
    let mut outputs = vec![Output {
        path: target.out.clone(),
        bytes: png(&image, scale)?,
    }];
    if json {
        let layers: Vec<&str> = target.layers.iter().map(String::as_str).collect();
        let image_name = target
            .out
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let tags = match job.frames {
            FrameScope::All => document.loaded().map_or_else(Vec::new, |loaded| {
                frame_tags(
                    &loaded.timeline.markers,
                    scene.timing().fps.get(),
                    scene.timing().duration.get(),
                )
            }),
            FrameScope::Current(_) => Vec::new(),
        };
        let tags: Vec<FrameTag<'_>> = tags
            .iter()
            .map(|(name, from, to)| FrameTag {
                name,
                from: *from,
                to: *to,
            })
            .collect();
        let frame = frames
            .first()
            .map_or((scene.size().width(), scene.size().height()), |first| {
                (
                    u16::try_from(first.width).unwrap_or(u16::MAX),
                    u16::try_from(first.height).unwrap_or(u16::MAX),
                )
            });
        let meta = SheetMeta {
            title: scene.name(),
            image: &image_name,
            size: (image.width, image.height),
            scale: job.scale,
            frame,
            cells: &cells,
            duration_ms: scene.timing().fps.period(1000),
            layers: &layers,
            trim,
            tags: &tags,
        };
        outputs.push(Output {
            path: target.out.with_extension("json"),
            bytes: px_export::aseprite_json(&meta).into_bytes(),
        });
    }
    Ok(outputs)
}
