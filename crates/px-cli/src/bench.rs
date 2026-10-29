use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Result;
use px_core::Frame;
use px_director::{Player, Sequence};
use px_engine::{FrameBuffer, Scene};
use px_format::{LoadOptions, Work};

use crate::args::Sweep;
use crate::source;

mod table;

use table::Table;

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
    p95: Duration,
    worst: Duration,
    total: Duration,
}

/// What a bench row reports about a scene, or about the largest shot of a sequence.
struct Counts {
    size: String,
    frames: u32,
    shapes: usize,
    instances: usize,
    lights: usize,
}

impl Counts {
    fn of(scene: &Scene, frames: u32) -> Self {
        let stats = scene.stats();
        Self {
            size: format!("{}x{}", scene.size().width(), scene.size().height()),
            frames,
            shapes: stats.shapes,
            instances: stats.instances,
            lights: stats.lights,
        }
    }

    fn of_sequence(sequence: &Sequence) -> Self {
        let frames = sequence.duration().get();
        let mut counts = Self::of(&sequence.shots()[0].scene, frames);
        for shot in sequence.shots() {
            let shot = Self::of(&shot.scene, frames);
            counts.shapes = counts.shapes.max(shot.shapes);
            counts.instances = counts.instances.max(shot.instances);
            counts.lights = counts.lights.max(shot.lights);
        }
        counts
    }
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
    let jobs: Vec<_> = files
        .iter()
        .flat_map(|file| {
            runs.iter()
                .map(move |run| (file, run, label(file, run.as_ref())))
        })
        .collect();
    let mut table = Table::new(jobs.iter().map(|(_, _, label)| label.as_str()), budget);
    let mut over = Vec::new();
    for (file, run, label) in jobs {
        let params: Vec<(String, String)> = run.iter().cloned().collect();
        let options = LoadOptions {
            params: &params,
            ..LoadOptions::default()
        };
        let (counts, timing) = match source::load_work_file(file, options)? {
            Work::Scene(loaded) => {
                let scene = &loaded.scene;
                let mut buffer = FrameBuffer::new(scene);
                let timing = time_frames(scene.timing().duration.get(), rounds, |frame| {
                    buffer.render(scene, frame).map(drop)
                })?;
                (Counts::of(scene, scene.timing().duration.get()), timing)
            }
            Work::Sequence(loaded) => {
                let sequence = &loaded.sequence;
                let mut player = Player::new(sequence);
                let timing = time_frames(sequence.duration().get(), rounds, |frame| {
                    player.render(sequence, frame).map(drop)
                })?;
                (Counts::of_sequence(sequence), timing)
            }
        };
        table.row(&label, &counts, &timing);
        if timing.worst > budget {
            over.push(label);
        }
    }
    table.finish();
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

fn label(file: &Path, run: Option<&(String, String)>) -> String {
    match run {
        Some((param, value)) => format!("{} [{param}={value}]", file.display()),
        None => file.display().to_string(),
    }
}

/// Times every frame, keeping each frame's best of `rounds` renders.
fn time_frames<E>(
    frames: u32,
    rounds: u32,
    mut render: impl FnMut(Frame) -> Result<(), E>,
) -> Result<Timing>
where
    E: std::error::Error + Send + Sync + 'static,
{
    let mut times = Vec::with_capacity(usize::try_from(frames).unwrap_or(0));
    for frame in (0..frames).map(Frame::new) {
        let mut best = Duration::MAX;
        for _ in 0..rounds.max(1) {
            let start = Instant::now();
            render(frame)?;
            best = best.min(start.elapsed());
        }
        times.push(best);
    }
    let total: Duration = times.iter().sum();
    times.sort_unstable();
    let rank = |share: usize| {
        times
            .len()
            .saturating_mul(share)
            .div_ceil(100)
            .saturating_sub(1)
    };
    Ok(Timing {
        mean: total / frames.max(1),
        p95: times.get(rank(95)).copied().unwrap_or_default(),
        worst: times.last().copied().unwrap_or_default(),
        total,
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
