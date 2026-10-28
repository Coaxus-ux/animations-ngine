use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use px_core::Frame;
use px_engine::{FrameBuffer, FrameReport, Scene};
use px_export::{GifWriter, MAX_SCALE, Scale, write_png};

use crate::args::{Command, Input};
use crate::source;

const OUTPUT_DIR: &str = "out";

pub(crate) fn run(command: &Command) -> Result<()> {
    match command {
        Command::Render { input, frame, out } => render(input, Frame::new(*frame), out.as_deref()),
        Command::Export { input, out } => export(input, out.as_deref()),
        Command::Watch { input } => crate::watch::watch(input, scale(input)?),
    }
}

fn render(input: &Input, frame: Frame, out: Option<&Path>) -> Result<()> {
    let scene = source::load(input)?;
    let scale = scale(input)?;
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
    println!("wrote {}", path.display());
    Ok(())
}

fn export(input: &Input, out: Option<&Path>) -> Result<()> {
    let scene = source::load(input)?;
    let scale = scale(input)?;
    let path = output_path(out, &format!("{}.gif", scene.name()))?;
    write_gif(&scene, scale, &path)?;
    println!(
        "wrote {} ({} frames)",
        path.display(),
        scene.timing().duration.get()
    );
    Ok(())
}

fn write_gif(scene: &Scene, scale: Scale, path: &Path) -> Result<()> {
    let timing = scene.timing();
    let file = BufWriter::new(create(path)?);
    let mut gif = GifWriter::new(file, scene.size(), scene.palette(), scale, timing.fps)?;
    let mut buffer = FrameBuffer::new(scene);
    let mut total = FrameReport::default();
    for frame in timing.duration.frames() {
        gif.add_frame(&buffer.render(scene, frame)?)?;
        let report = buffer.report();
        total.non_finite_values = total
            .non_finite_values
            .saturating_add(report.non_finite_values);
        total.first_non_finite_shape = total
            .first_non_finite_shape
            .or(report.first_non_finite_shape);
    }
    warn_non_finite(total);
    Ok(())
}

fn warn_non_finite(report: FrameReport) {
    if let Some(shape) = report.first_non_finite_shape {
        eprintln!(
            "warning: {} non-finite value(s) were replaced by 0, first in shape #{shape}",
            report.non_finite_values
        );
    }
}

fn scale(input: &Input) -> Result<Scale> {
    Scale::new(input.scale).with_context(|| format!("--scale must be 1..={MAX_SCALE}"))
}

fn output_path(out: Option<&Path>, default_name: &str) -> Result<PathBuf> {
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

fn create(path: &Path) -> Result<File> {
    File::create(path).with_context(|| format!("cannot create {}", path.display()))
}
