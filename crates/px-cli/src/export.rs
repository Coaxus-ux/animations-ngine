mod flags;
mod job;

use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use clap::{Args, ValueEnum};
use px_editor::domain::export::{ExportKind, KIND_NAMES, LAYOUT_NAMES};
use px_format::Work;

use crate::args::Input;
use crate::commands::{output_path, warn_non_finite};
use crate::source;

#[derive(Debug, Args)]
pub(crate) struct Options {
    #[arg(
        long,
        help = "Output path (default: `out/<scene>.<ext>`). A sequence numbers it per frame; a sheet's JSON goes next to it"
    )]
    pub(crate) out: Option<PathBuf>,
    #[arg(
        long,
        help = "Threads that render and compress GIF frames (default: every core); a GIF without --transparent or --frame only. The GIF is byte-identical for any value"
    )]
    pub(crate) jobs: Option<NonZeroUsize>,
    #[arg(long, value_parser = kind_index, default_value = KIND_NAMES[0], help = "What to write: a looping GIF, one frame as PNG, a numbered PNG sequence or a sprite sheet")]
    pub(crate) format: usize,
    #[arg(long, value_parser = layout_index, help = "Sprite sheet layout: strip-h, strip-v, rows or columns (default: strip-h)")]
    pub(crate) layout: Option<usize>,
    #[arg(long, help = "Columns of a `rows` sheet, rows of a `columns` sheet")]
    pub(crate) count: Option<u16>,
    #[arg(long, help = "Empty pixels between sheet frames")]
    pub(crate) padding: Option<u16>,
    #[arg(long, help = "Empty pixels around the sheet")]
    pub(crate) border: Option<u16>,
    #[arg(long, help = "Transparent background by keyed render (D-018)")]
    pub(crate) transparent: bool,
    #[arg(long, value_enum, help = "Sheet metadata file (default: json)")]
    pub(crate) meta: Option<Meta>,
    #[arg(
        long,
        help = "The frame to export (png: default 0); other formats export every frame unless given"
    )]
    pub(crate) frame: Option<u32>,
    #[arg(
        long,
        help = "Trim each frame to its opaque pixels (png, sequence or sheet)"
    )]
    pub(crate) trim: bool,
    #[arg(long, help = "One output per layer (sequence or sheet)")]
    pub(crate) split_layers: bool,
}

fn index_in(text: &str, names: &[&str]) -> Result<usize, String> {
    names
        .iter()
        .position(|name| *name == text)
        .ok_or_else(|| format!("expected one of {}", names.join(", ")))
}

fn kind_index(text: &str) -> Result<usize, String> {
    index_in(text, &KIND_NAMES)
}

fn layout_index(text: &str) -> Result<usize, String> {
    index_in(text, &LAYOUT_NAMES)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub(crate) enum Meta {
    Json,
    None,
}

pub(crate) fn export(input: &Input, options: &Options) -> Result<()> {
    if is_plain_gif(options) {
        return plain_gif(input, options);
    }
    if !input.params.is_empty() {
        bail!("params are not supported for this format yet");
    }
    let outputs = job::run(input, options)?;
    for path in outputs {
        println!("wrote {}", path.display());
    }
    Ok(())
}

fn is_plain_gif(options: &Options) -> bool {
    flags::plain_kind(options.format) == ExportKind::Gif && flags::editor_only(options).is_empty()
}

fn plain_gif(input: &Input, options: &Options) -> Result<()> {
    flags::sheet_only(options)?;
    let scale = crate::commands::scale(input)?;
    let jobs = options.jobs.unwrap_or_else(crate::parallel::default_jobs);
    let out = options.out.as_deref();
    let (frames, path) = match source::load_work(input)? {
        Work::Scene(loaded) => {
            let path = output_path(out, &format!("{}.gif", loaded.scene.name()))?;
            warn_non_finite(crate::gif::write_scene(&loaded.scene, scale, jobs, &path)?);
            (loaded.scene.timing().duration.get(), path)
        }
        Work::Sequence(loaded) => {
            let path = output_path(out, &format!("{}.gif", loaded.sequence.name()))?;
            crate::gif::write_sequence(&loaded.sequence, scale, jobs, &path)?;
            (loaded.sequence.duration().get(), path)
        }
    };
    println!("wrote {} ({frames} frames)", path.display());
    Ok(())
}

fn write(path: &Path, bytes: &[u8]) -> Result<PathBuf> {
    let path = output_path(Some(path), "")?;
    std::fs::write(&path, bytes)
        .map_err(|error| anyhow::anyhow!("cannot write {}: {error}", path.display()))?;
    Ok(path)
}
