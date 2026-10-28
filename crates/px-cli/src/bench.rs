use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Result;
use px_engine::{FrameBuffer, Scene};
use px_format::LoadOptions;

use crate::args::Sweep;
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

pub(crate) fn bench(
    files: &[PathBuf],
    budget_ms: f64,
    rounds: u32,
    sweep: Option<&Sweep>,
) -> Result<()> {
    let files = if files.is_empty() {
        default_files()
    } else {
        files.to_vec()
    };
    let budget = Duration::from_secs_f64(budget_ms.max(0.0) / 1000.0);
    let runs: Vec<Option<(String, String)>> = match sweep {
        Some(sweep) => sweep
            .values
            .iter()
            .map(|value| Some((sweep.param.clone(), value.clone())))
            .collect(),
        None => vec![None],
    };
    let mut over = Vec::new();
    println!(
        "{:<44} {:>9} {:>6} {:>6} {:>5} {:>6} {:>8} {:>8}",
        "scene", "size", "frames", "shapes", "inst", "lights", "mean ms", "worst ms"
    );
    for file in &files {
        for run in &runs {
            let label = match run {
                Some((param, value)) => format!("{} [{param}={value}]", file.display()),
                None => file.display().to_string(),
            };
            let params: Vec<(String, String)> = run.iter().cloned().collect();
            let options = LoadOptions {
                params: &params,
                ..LoadOptions::default()
            };
            let scene = source::load_file(file, options).0?.scene;
            let timing = time_frames(&scene, rounds.max(1))?;
            let late = timing.worst > budget;
            print_row(&label, &scene, &timing, late);
            if late {
                over.push(label);
            }
        }
    }
    match (over.first(), sweep) {
        (None, _) => Ok(()),
        (Some(first), Some(_)) => {
            println!("first over the {budget_ms:.1} ms budget: {first}");
            Ok(())
        }
        (Some(_), None) => Err(OverBudget {
            scenes: over,
            budget_ms,
        }
        .into()),
    }
}

fn print_row(label: &str, scene: &Scene, timing: &Timing, late: bool) {
    let stats = scene.stats();
    let size = format!("{}x{}", scene.size().width(), scene.size().height());
    let verdict = if late { "over budget" } else { "" };
    println!(
        "{label:<44} {size:>9} {:>6} {:>6} {:>5} {:>6} {:>8.3} {:>8.3} {verdict}",
        scene.timing().duration.get(),
        stats.shapes,
        stats.instances,
        stats.lights,
        timing.mean.as_secs_f64() * 1000.0,
        timing.worst.as_secs_f64() * 1000.0,
    );
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
