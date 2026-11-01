use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use px_core::Frame;
use px_engine::{FrameBuffer, FrameReport};
use px_export::{MAX_SCALE, Scale, write_png};
use px_format::Work;

use crate::args::{Command, Input};
use crate::source;

pub(crate) const OUTPUT_DIR: &str = "out";

pub(crate) fn run(command: &Command) -> Result<()> {
    match command {
        Command::Render { input, frame, out } => render(input, Frame::new(*frame), out.as_deref()),
        Command::Export { input, options } => crate::export::export(input, options),
        Command::Watch { input } => crate::watch::watch(input, scale(input)?),
        Command::Edit { file, size, scale } => crate::edit::edit(file.as_deref(), *size, *scale),
        Command::Timeline { input } => crate::timeline::print(input),
        Command::Bench {
            files,
            budget_ms,
            rounds,
            sweep,
        } => crate::bench::bench(files, *budget_ms, *rounds, sweep.as_ref()),
    }
}

fn render(input: &Input, frame: Frame, out: Option<&Path>) -> Result<()> {
    let scale = scale(input)?;
    let (name, path) = match source::load_work(input)? {
        Work::Scene(loaded) => {
            let scene = loaded.scene;
            let path = output_path(out, &format!("{}-{}.png", scene.name(), frame.index()))?;
            let mut buffer = FrameBuffer::new(&scene);
            let view = buffer.render(&scene, frame)?;
            write_png(
                BufWriter::new(create(&path)?),
                &view,
                scene.palette(),
                scale,
            )?;
            warn_non_finite(buffer.report());
            (scene.name().to_owned(), path)
        }
        Work::Sequence(loaded) => {
            let sequence = loaded.sequence;
            let path = output_path(out, &format!("{}-{}.png", sequence.name(), frame.index()))?;
            crate::sequence::write_png(&sequence, frame, scale, &path)?;
            (sequence.name().to_owned(), path)
        }
    };
    println!("wrote {} ({name})", path.display());
    Ok(())
}

pub(crate) fn warn_non_finite(report: FrameReport) {
    if let Some(shape) = report.first_non_finite_shape {
        eprintln!(
            "warning: {} non-finite value(s) were replaced by 0, first in shape #{shape}",
            report.non_finite_values
        );
    }
}

pub(crate) fn scale(input: &Input) -> Result<Scale> {
    Scale::new(input.scale).with_context(|| format!("--scale must be 1..={MAX_SCALE}"))
}

pub(crate) fn output_path(out: Option<&Path>, default_name: &str) -> Result<PathBuf> {
    let path = out.map_or_else(
        || Path::new(OUTPUT_DIR).join(default_name),
        Path::to_path_buf,
    );
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    Ok(path)
}

pub(crate) fn create(path: &Path) -> Result<File> {
    File::create(path).with_context(|| format!("cannot create {}", path.display()))
}
