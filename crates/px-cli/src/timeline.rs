use std::fmt::Write as _;

use anyhow::Result;
use px_format::{Timeline, Track};

use crate::args::Input;
use crate::source;

const COLUMNS: usize = 48;
const LABEL: usize = 24;

pub(crate) fn print(input: &Input) -> Result<()> {
    let loaded = source::load_tracked(input).0?;
    print!("{}", chart(loaded.scene.name(), &loaded.timeline));
    Ok(())
}

fn chart(name: &str, timeline: &Timeline) -> String {
    let duration = timeline.duration.max(f64::EPSILON);
    let column = |seconds: f64| -> usize {
        let share = (seconds / duration).clamp(0.0, 1.0) * to_f64(COLUMNS);
        to_usize(share.floor()).min(COLUMNS - 1)
    };
    let mut text = String::new();
    let looping = if timeline.looping { ", loops" } else { "" };
    let _ = writeln!(text, "{name} ({:.2}s{looping})", timeline.duration);
    let mut ruler = vec![b'-'; COLUMNS];
    let mut labels = vec![b' '; COLUMNS + 8];
    for step in 0..=4 {
        let seconds = timeline.duration * f64::from(step) / 4.0;
        let at = column(seconds);
        ruler[at] = b'|';
        for (offset, byte) in format!("{seconds:.1}s").bytes().enumerate() {
            if let Some(slot) = labels.get_mut(at + offset) {
                *slot = byte;
            }
        }
    }
    let _ = writeln!(
        text,
        "{:LABEL$}{}",
        "",
        String::from_utf8_lossy(&labels).trim_end()
    );
    let _ = writeln!(text, "{:LABEL$}{}", "", String::from_utf8_lossy(&ruler));
    for marker in &timeline.markers {
        let mut row = vec![b' '; COLUMNS];
        row[column(marker.at)] = b'^';
        let label = format!("marker {}", marker.name);
        let _ = writeln!(
            text,
            "{label:LABEL$}{} {:.2}s",
            String::from_utf8_lossy(&row),
            marker.at
        );
    }
    for track in &timeline.tracks {
        let _ = writeln!(text, "{}", bar(track, timeline.duration, column));
    }
    text
}

fn bar(track: &Track, duration: f64, column: impl Fn(f64) -> usize) -> String {
    let mut label = format!("{}{}", "  ".repeat(track.depth), track.scene);
    if let Some(id) = &track.id {
        let _ = write!(label, " #{id}");
    }
    if track.count != 1 {
        let _ = write!(label, " ×{}", track.count);
    }
    let first = column(track.start);
    let last = if track.end >= duration {
        COLUMNS
    } else {
        column(track.end)
    };
    let bar: String = (0..COLUMNS)
        .map(|at| {
            if (first..last).contains(&at) {
                '='
            } else {
                '.'
            }
        })
        .collect();
    let mode = if track.looping { "loop" } else { "once" };
    format!(
        "{label:LABEL$}{bar} {mode} {:.2}s ×{:.2} +{:.2}s",
        track.duration, track.speed, track.offset
    )
}

#[expect(clippy::cast_precision_loss, reason = "column counts are tiny")]
const fn to_f64(value: usize) -> f64 {
    value as f64
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped to the column range first"
)]
const fn to_usize(value: f64) -> usize {
    value as usize
}

#[cfg(test)]
mod tests {
    use px_format::{Marker, Timeline, Track};

    use super::chart;

    #[test]
    fn bars_cover_the_window_each_scene_plays_in() {
        let track = |scene: &str, start: f64, end: f64, depth| Track {
            scene: scene.to_owned(),
            id: None,
            depth,
            count: 1,
            start,
            end,
            offset: 0.0,
            speed: 1.0,
            duration: 1.0,
            looping: true,
        };
        let timeline = Timeline {
            duration: 8.0,
            looping: true,
            markers: vec![Marker {
                name: "half".to_owned(),
                at: 4.0,
            }],
            tracks: vec![
                track("backdrop", 0.0, f64::INFINITY, 0),
                track("lantern", 4.0, 6.0, 1),
            ],
        };
        let text = chart("town", &timeline);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "town (8.00s, loops)");
        assert!(lines[3].starts_with("marker half") && lines[3].contains('^'));
        assert!(lines[4].contains(&"=".repeat(48)));
        let lantern = lines[5];
        assert!(lantern.starts_with("  lantern"));
        assert_eq!(lantern.matches('=').count(), 12, "{lantern}");
    }
}
