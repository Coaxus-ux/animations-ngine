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

/// `pxe export` flags beyond the input. The names mirror the editor's layout file
/// (`gif|png|sequence|sheet`, `strip-h|strip-v|rows|columns`, D-023).
#[derive(Debug, Args)]
pub(crate) struct Options {
    /// Output path (default: `out/<scene>.<ext>`). A sequence numbers it per frame; a
    /// sheet's JSON goes next to it.
    #[arg(long)]
    pub(crate) out: Option<PathBuf>,
    /// Threads that render and compress GIF frames (default: every core); a GIF without
    /// --transparent or --frame only. The GIF is byte-identical for any value.
    #[arg(long)]
    pub(crate) jobs: Option<NonZeroUsize>,
    /// What to write: a looping GIF, one frame as PNG, a numbered PNG sequence or a
    /// sprite sheet.
    #[arg(long, value_parser = kind_index, default_value = KIND_NAMES[0])]
    pub(crate) format: usize,
    /// Sprite sheet layout: strip-h, strip-v, rows or columns (default: strip-h).
    #[arg(long, value_parser = layout_index)]
    pub(crate) layout: Option<usize>,
    /// Columns of a `rows` sheet, rows of a `columns` sheet.
    #[arg(long)]
    pub(crate) count: Option<u16>,
    /// Empty pixels between sheet frames.
    #[arg(long)]
    pub(crate) padding: Option<u16>,
    /// Empty pixels around the sheet.
    #[arg(long)]
    pub(crate) border: Option<u16>,
    /// Transparent background by keyed render (D-018).
    #[arg(long)]
    pub(crate) transparent: bool,
    /// Sheet metadata file (default: json).
    #[arg(long, value_enum)]
    pub(crate) meta: Option<Meta>,
    /// The frame to export (png: default 0); other formats export every frame unless given.
    #[arg(long)]
    pub(crate) frame: Option<u32>,
    /// Trim each frame to its opaque pixels (png, sequence or sheet).
    #[arg(long)]
    pub(crate) trim: bool,
    /// One output per layer (sequence or sheet).
    #[arg(long)]
    pub(crate) split_layers: bool,
}

/// The position of `text` in `names`, the editor's kind and layout tables, so the CLI
/// accepts exactly what the editor's layout file does.
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

/// A GIF with no editor-only option takes the parallel delta encoder (D-010).
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
