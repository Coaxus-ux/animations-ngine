use std::collections::BTreeSet;
use std::path::PathBuf;

use anyhow::{Result, anyhow, bail};
use px_editor::domain::Document;
use px_editor::domain::export::{self, ExportJob, ExportKind, FrameScope};
use px_export::Padding;

use super::{Meta, Options, flags};
use crate::args::Input;
use crate::commands::OUTPUT_DIR;
use crate::edit::files::DiskFiles;
use crate::source;

pub(super) fn run(input: &Input, options: &Options) -> Result<Vec<PathBuf>> {
    let kind = kind(options)?;
    let document =
        Document::open(&input.file, &mut DiskFiles).map_err(|reason| anyhow!("{reason}"))?;
    let scene = document.scene().ok_or_else(|| {
        anyhow!(
            "{} could not be loaded: {}",
            input.file.display(),
            document.errors().join("; ")
        )
    })?;
    if let Some(wanted) = &input.scene
        && wanted != scene.name()
    {
        source::load_work(input)?;
        bail!(
            "--scene {wanted}: only the file's first scene (\"{}\") can be exported {} yet",
            scene.name(),
            flags::restriction(options)
        );
    }
    let frames = frames(options);
    let job = ExportJob {
        kind,
        scale: input.scale,
        detail: input.detail,
        transparent: options.transparent,
        frames,
        out: options
            .out
            .clone()
            .unwrap_or_else(|| default_out(scene.name(), kind, frames)),
        trim: options.trim,
        split_layers: options.split_layers,
    };
    let outputs =
        export::run(&job, &document, &BTreeSet::new()).map_err(|reason| anyhow!("{reason}"))?;
    outputs
        .iter()
        .map(|output| super::write(&output.path, &output.bytes))
        .collect()
}

fn kind(options: &Options) -> Result<ExportKind> {
    if !matches!(flags::plain_kind(options.format), ExportKind::Sheet { .. }) {
        flags::sheet_only(options)?;
    }
    let kind = ExportKind::from_index(
        options.format,
        flags::layout(options)?,
        Padding {
            border: options.border.unwrap_or_default(),
            shape: options.padding.unwrap_or_default(),
        },
        options.meta.unwrap_or(Meta::Json) == Meta::Json,
    );
    flags::for_kind(options, kind)?;
    Ok(kind)
}

fn frames(options: &Options) -> FrameScope {
    match (flags::plain_kind(options.format), options.frame) {
        (ExportKind::Png, frame) => FrameScope::Current(frame.unwrap_or(0)),
        (_, Some(frame)) => FrameScope::Current(frame),
        (_, None) => FrameScope::All,
    }
}

fn default_out(scene: &str, kind: ExportKind, frames: FrameScope) -> PathBuf {
    let name = match (kind, frames) {
        (ExportKind::Png, FrameScope::Current(frame)) => format!("{scene}-{frame}.png"),
        _ => format!("{scene}.{}", kind.extension()),
    };
    PathBuf::from(OUTPUT_DIR).join(name)
}
