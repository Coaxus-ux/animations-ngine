mod kdl;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use px_engine::{FrameBuffer, Scene};
use px_export::{GifWriter, Padding, RgbaImage, Scale, SheetLayout, SheetMeta};

use super::Document;
use super::frames::{frames_in, message, render_frames, visible_scene};

/// The dialog's and the layout file's order of kinds; `ExportKind::index` is its inverse.
pub const KIND_LABELS: [&str; 4] = ["GIF", "PNG", "PNG sequence", "Sprite sheet"];
/// The dialog's and the layout file's order of sheet layouts; `layout_index` is its inverse.
pub const LAYOUT_LABELS: [&str; 4] = ["Strip H", "Strip V", "Rows", "Columns"];
pub const MAX_SCALE: u8 = px_export::MAX_SCALE;
/// Sequence suffixes are zero-padded to the widest index a scene can reach, so file
/// names sort in frame order for every frame count.
pub const FRAME_INDEX_DIGITS: usize = digits(px_core::MAX_FRAMES - 1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportKind {
    Gif,
    /// The current frame as one PNG.
    Png,
    /// One PNG per frame, numbered.
    PngSequence,
    Sheet {
        layout: SheetLayout,
        padding: Padding,
        /// Also write Aseprite JSON next to the image.
        json: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameScope {
    All,
    Current(u32),
}

/// Everything the Export dialog decides; kept as the last export for Repeat Last Export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportJob {
    pub kind: ExportKind,
    pub scale: u8,
    pub transparent: bool,
    pub frames: FrameScope,
    pub out: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
}

impl Default for ExportJob {
    fn default() -> Self {
        Self {
            kind: ExportKind::Sheet {
                layout: SheetLayout::HorizontalStrip,
                padding: Padding::default(),
                json: true,
            },
            scale: 1,
            transparent: true,
            frames: FrameScope::All,
            out: PathBuf::new(),
        }
    }
}

impl ExportKind {
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Gif => 0,
            Self::Png => 1,
            Self::PngSequence => 2,
            Self::Sheet { .. } => 3,
        }
    }

    /// The kind at `index` in `KIND_LABELS`; a sheet takes its settings from the rest.
    #[must_use]
    pub fn from_index(index: usize, layout: SheetLayout, padding: Padding, json: bool) -> Self {
        match index {
            0 => Self::Gif,
            1 => Self::Png,
            2 => Self::PngSequence,
            _ => Self::Sheet {
                layout,
                padding,
                json,
            },
        }
    }

    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Gif => "gif",
            Self::Png | Self::PngSequence | Self::Sheet { .. } => "png",
        }
    }
}

#[must_use]
pub const fn layout_index(layout: SheetLayout) -> usize {
    match layout {
        SheetLayout::HorizontalStrip => 0,
        SheetLayout::VerticalStrip => 1,
        SheetLayout::Rows { .. } => 2,
        SheetLayout::Columns { .. } => 3,
    }
}

/// The columns of a `Rows` layout or the rows of a `Columns` one; strips have none.
#[must_use]
pub const fn layout_count(layout: SheetLayout) -> u16 {
    match layout {
        SheetLayout::HorizontalStrip | SheetLayout::VerticalStrip => 1,
        SheetLayout::Rows { columns } => columns,
        SheetLayout::Columns { rows } => rows,
    }
}

/// The layout at `index` in `LAYOUT_LABELS` with `count` columns or rows.
#[must_use]
pub const fn layout_from(index: usize, count: u16) -> SheetLayout {
    match index {
        0 => SheetLayout::HorizontalStrip,
        1 => SheetLayout::VerticalStrip,
        2 => SheetLayout::Rows { columns: count },
        _ => SheetLayout::Columns { rows: count },
    }
}

/// Renders and encodes `job`; nothing is written. Hidden layers stay out.
///
/// # Errors
/// A readable reason: the scene doesn't load, the scale is out of range, or the encoder
/// refused.
pub fn run(
    job: &ExportJob,
    document: &Document,
    hidden: &BTreeSet<String>,
) -> Result<Vec<Output>, String> {
    let scale = Scale::new(job.scale).ok_or_else(|| format!("scale must be 1..={MAX_SCALE}"))?;
    match job.kind {
        ExportKind::Gif => Ok(vec![Output {
            path: job.out.clone(),
            bytes: gif(&visible_scene(document, hidden)?, scale, job.frames)?,
        }]),
        ExportKind::Png => {
            if job.frames == FrameScope::All {
                return Err("PNG exports one frame: pick Current".to_owned());
            }
            let frames = render_frames(document, hidden, job.transparent, job.frames)?;
            let image = frames
                .first()
                .ok_or_else(|| "no frame to export".to_owned())?;
            Ok(vec![Output {
                path: job.out.clone(),
                bytes: png(image, scale)?,
            }])
        }
        ExportKind::PngSequence => {
            let frames = render_frames(document, hidden, job.transparent, job.frames)?;
            let stem = stem(&job.out);
            frames
                .iter()
                .enumerate()
                .map(|(index, image)| {
                    Ok(Output {
                        path: job.out.with_file_name(sequence_name(&stem, index)),
                        bytes: png(image, scale)?,
                    })
                })
                .collect()
        }
        ExportKind::Sheet {
            layout,
            padding,
            json,
        } => sheet(job, document, hidden, scale, layout, padding, json),
    }
}

fn gif(scene: &Scene, scale: Scale, scope: FrameScope) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let timing = scene.timing();
    let mut gif = GifWriter::new(&mut bytes, scene.size(), scene.palette(), scale, timing.fps)
        .map_err(message)?;
    let mut buffer = FrameBuffer::new(scene);
    for frame in frames_in(scene, scope) {
        let view = buffer.render(scene, frame).map_err(message)?;
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

fn sheet(
    job: &ExportJob,
    document: &Document,
    hidden: &BTreeSet<String>,
    scale: Scale,
    layout: SheetLayout,
    padding: Padding,
    json: bool,
) -> Result<Vec<Output>, String> {
    let frames = render_frames(document, hidden, job.transparent, job.frames)?;
    let (image, cells) = px_export::compose(&frames, layout, padding).map_err(message)?;
    let mut outputs = vec![Output {
        path: job.out.clone(),
        bytes: png(&image, scale)?,
    }];
    if json {
        let scene = visible_scene(document, hidden)?;
        let layers: Vec<&str> = document
            .layers()
            .iter()
            .filter(|layer| !hidden.contains(&layer.name))
            .map(|layer| layer.name.as_str())
            .collect();
        let image_name = job
            .out
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let meta = SheetMeta {
            title: scene.name(),
            image: &image_name,
            size: (image.width, image.height),
            scale: job.scale,
            frame: (scene.size().width(), scene.size().height()),
            cells: &cells,
            duration_ms: scene.timing().fps.period(1000),
            layers: &layers,
        };
        outputs.push(Output {
            path: job.out.with_extension("json"),
            bytes: px_export::aseprite_json(&meta).into_bytes(),
        });
    }
    Ok(outputs)
}

/// `<stem>-<index>.png` with the index padded to `FRAME_INDEX_DIGITS`.
#[must_use]
pub fn sequence_name(stem: &str, index: usize) -> String {
    format!("{stem}-{index:0FRAME_INDEX_DIGITS$}.png")
}

const fn digits(mut value: u32) -> usize {
    let mut count = 1;
    while value >= 10 {
        value /= 10;
        count += 1;
    }
    count
}

fn stem(path: &Path) -> String {
    path.file_stem().map_or_else(
        || "frame".to_owned(),
        |stem| stem.to_string_lossy().into_owned(),
    )
}

/// How many frames `scope` exports from a scene `duration` frames long.
#[must_use]
pub fn frame_count(scope: FrameScope, duration: u32) -> usize {
    match scope {
        FrameScope::All => usize::try_from(duration).unwrap_or(usize::MAX),
        FrameScope::Current(_) => 1,
    }
}
