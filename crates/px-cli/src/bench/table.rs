use std::fmt::Write as _;
use std::time::Duration;

use super::{Counts, Timing};

const COLUMNS: [(&str, usize); 10] = [
    ("size", 9),
    ("frames", 6),
    ("shapes", 6),
    ("inst", 5),
    ("lights", 6),
    ("mean ms", 8),
    ("p95 ms", 8),
    ("worst ms", 8),
    ("fps", 6),
    ("budget", 7),
];

pub(super) struct Table {
    label_width: usize,
    scenes: usize,
    frames: u64,
    total: Duration,
    slowest: Option<(Duration, String)>,
    over: usize,
}

impl Table {
    pub(super) fn new<'a>(labels: impl Iterator<Item = &'a str>) -> Self {
        let label_width = labels.map(str::len).chain([5]).max().unwrap_or(5);
        let table = Self {
            label_width,
            scenes: 0,
            frames: 0,
            total: Duration::ZERO,
            slowest: None,
            over: 0,
        };
        let headers = COLUMNS.map(|(header, _)| header.to_owned());
        table.line("scene", &headers, "");
        table.rule();
        table
    }

    pub(super) fn row(&mut self, label: &str, counts: &Counts, timing: &Timing, budget: Duration) {
        let late = timing.worst > budget;
        let cells = [
            counts.size.clone(),
            counts.frames.to_string(),
            counts.shapes.to_string(),
            counts.instances.to_string(),
            counts.lights.to_string(),
            milliseconds(timing.mean),
            milliseconds(timing.p95),
            milliseconds(timing.worst),
            format!("{:.0}", 1.0 / timing.mean.as_secs_f64().max(f64::EPSILON)),
            format!(
                "{:.0}%",
                100.0 * timing.worst.as_secs_f64() / budget.as_secs_f64().max(f64::EPSILON)
            ),
        ];
        self.line(label, &cells, if late { "  over budget" } else { "" });
        self.scenes += 1;
        self.frames += u64::from(counts.frames);
        self.total += timing.total;
        if self
            .slowest
            .as_ref()
            .is_none_or(|(worst, _)| timing.worst > *worst)
        {
            self.slowest = Some((timing.worst, label.to_owned()));
        }
        self.over += usize::from(late);
    }

    pub(super) fn finish(&self) {
        self.rule();
        let over = match self.over {
            0 => "none".to_owned(),
            count => count.to_string(),
        };
        let slowest = self
            .slowest
            .as_ref()
            .map(|(worst, label)| format!("; slowest frame {} ms in {label}", milliseconds(*worst)))
            .unwrap_or_default();
        println!(
            "{} scenes, {} frames, {:.2} s of frame time{slowest}; {over} over budget",
            self.scenes,
            self.frames,
            self.total.as_secs_f64(),
        );
    }

    fn line(&self, label: &str, cells: &[String; COLUMNS.len()], suffix: &str) {
        let mut line = format!("{label:<width$}", width = self.label_width);
        for (cell, (_, width)) in cells.iter().zip(COLUMNS) {
            let _ = write!(line, "  {cell:>width$}");
        }
        println!("{line}{suffix}");
    }

    fn rule(&self) {
        let width = self.label_width + COLUMNS.iter().map(|(_, width)| width + 2).sum::<usize>();
        println!("{}", "-".repeat(width));
    }
}

fn milliseconds(duration: Duration) -> String {
    format!("{:.2}", duration.as_secs_f64() * 1000.0)
}
