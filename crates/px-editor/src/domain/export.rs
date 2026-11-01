mod kdl;
mod outputs;
mod tags;
mod targets;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub use kdl::{KIND_NAMES, LAYOUT_NAMES};
use px_engine::Detail;
use px_export::{Padding, Scale, SheetLayout};
pub use tags::frame_tags;

use super::Document;
use targets::Target;

pub const KIND_LABELS: [&str; 4] = ["GIF", "PNG", "PNG sequence", "Sprite sheet"];
pub const LAYOUT_LABELS: [&str; 4] = ["Strip H", "Strip V", "Rows", "Columns"];
pub const MAX_SCALE: u8 = px_export::MAX_SCALE;
pub const FRAME_INDEX_DIGITS: usize = digits(px_core::MAX_FRAMES - 1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportKind {
    Gif,
    Png,
    PngSequence,
    Sheet {
        layout: SheetLayout,
        padding: Padding,
        json: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameScope {
    All,
    Current(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportJob {
    pub kind: ExportKind,
    pub scale: u8,
    pub detail: Detail,
    pub transparent: bool,
    pub trim: bool,
    pub split_layers: bool,
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
            detail: Detail::NATIVE,
            transparent: true,
            trim: false,
            split_layers: false,
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

    #[must_use]
    pub const fn splits(self) -> bool {
        matches!(self, Self::PngSequence | Self::Sheet { .. })
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

#[must_use]
pub const fn layout_count(layout: SheetLayout) -> u16 {
    match layout {
        SheetLayout::HorizontalStrip | SheetLayout::VerticalStrip => 1,
        SheetLayout::Rows { columns } => columns,
        SheetLayout::Columns { rows } => rows,
    }
}

#[must_use]
pub const fn layout_from(index: usize, count: u16) -> SheetLayout {
    match index {
        0 => SheetLayout::HorizontalStrip,
        1 => SheetLayout::VerticalStrip,
        2 => SheetLayout::Rows { columns: count },
        _ => SheetLayout::Columns { rows: count },
    }
}

pub fn run(
    job: &ExportJob,
    document: &Document,
    hidden: &BTreeSet<String>,
) -> Result<Vec<Output>, String> {
    let scale = Scale::new(job.scale).ok_or_else(|| format!("scale must be 1..={MAX_SCALE}"))?;
    if job.kind == ExportKind::Png && job.frames == FrameScope::All {
        return Err("PNG exports one frame: pick Current".to_owned());
    }
    let mut outputs = Vec::new();
    for target in Target::all(job, document, hidden) {
        outputs.extend(outputs::encode(job, document, &target, scale)?);
    }
    Ok(outputs)
}

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

#[must_use]
pub fn frame_count(scope: FrameScope, duration: u32) -> usize {
    match scope {
        FrameScope::All => usize::try_from(duration).unwrap_or(usize::MAX),
        FrameScope::Current(_) => 1,
    }
}
