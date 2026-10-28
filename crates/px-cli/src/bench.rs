use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Result;
use px_engine::{FrameBuffer, Scene};
use px_format::LoadOptions;

use crate::source;

#[derive(Debug)]
pub(crate) struct OverBudget {
    scenes: Vec<String>,
    budget_ms: f64,
}

impl fmt::Display for OverBudget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "over the {:.1} ms frame budget: {}",
            self.budget_ms,
            self.scenes.join(", ")
        )
    }
}

impl std::error::Error for OverBudget {}

struct Timing {
    mean: Duration,
    worst: Duration,
}

pub(crate) fn bench(files: &[PathBuf], budget_ms: f64, rounds: u32) -> Result<()> {
    let files = if files.is_empty() {
        default_files()
    } else {
        files.to_vec()
    };
    let budget = Duration::from_secs_f64(budget_ms.max(0.0) / 1000.0);
    let mut over = Vec::new();
    println!(
        "{:<36} {:>9} {:>7} {:>9} {:>9}",
        "scene", "size", "frames", "mean ms", "worst ms"
    );
    for file in &files {
        let scene = source::load_file(file, LoadOptions::default())?;
        let timing = time_frames(&scene, rounds.max(1))?;
        let verdict = if timing.worst > budget {
            "over budget"
        } else {
            ""
        };
        let size = format!("{}x{}", scene.size().width(), scene.size().height());
        println!(
            "{:<36} {size:>9} {:>7} {:>9.3} {:>9.3} {verdict}",
            file.display(),
            scene.timing().duration.get(),
            timing.mean.as_secs_f64() * 1000.0,
            timing.worst.as_secs_f64() * 1000.0,
        );
        if timing.worst > budget {
            over.push(file.display().to_string());
        }
    }
    if over.is_empty() {
        Ok(())
    } else {
        Err(OverBudget {
            scenes: over,
            budget_ms,
        }
        .into())
    }
}

fn time_frames(scene: &Scene, rounds: u32) -> Result<Timing> {
    let mut buffer = FrameBuffer::new(scene);
    let (mut total, mut worst) = (Duration::ZERO, Duration::ZERO);
    let frames = scene.timing().duration;
    for frame in frames.frames() {
        let mut best = Duration::MAX;
        for _ in 0..rounds {
            let start = Instant::now();
            buffer.render(scene, frame)?;
            best = best.min(start.elapsed());
        }
        total += best;
        worst = worst.max(best);
    }
    Ok(Timing {
        mean: total / frames.get(),
        worst,
    })
}

fn default_files() -> Vec<PathBuf> {
    let mut files = kdl_files(Path::new("examples"));
    for dir in entries(Path::new("benchmarks"))
        .into_iter()
        .filter(|path| path.is_dir())
    {
        files.extend(kdl_files(&dir));
    }
    files.sort();
    files
}

fn entries(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .collect()
        })
        .unwrap_or_default()
}

fn kdl_files(dir: &Path) -> Vec<PathBuf> {
    entries(dir)
        .into_iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "kdl"))
        .collect()
}
