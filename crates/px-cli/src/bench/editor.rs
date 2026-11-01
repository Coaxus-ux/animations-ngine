use std::path::Path;
use std::time::Instant;

use anyhow::{Result, anyhow};
use px_editor::app::{App, Intent};
use px_editor::assets::ThemeChoice;
use px_editor::domain::Document;
use px_engine::Detail;
use px_ui::Input;

use super::{Counts, Timing, summarize};
use crate::edit::files::DiskFiles;

pub(super) fn time_editor(
    file: &Path,
    (width, height): (u16, u16),
    rounds: u32,
    detail: Detail,
) -> Result<(Counts, Timing)> {
    let window = (usize::from(width), usize::from(height));
    let mut files = DiskFiles;
    let mut app = App::new(ThemeChoice::Dark).map_err(|error| anyhow!(error))?;
    app.open(file, None, &mut files)
        .map_err(|error| anyhow!("{}: {error}", file.display()))?;
    let main = app
        .window_ids()
        .next()
        .ok_or_else(|| anyhow!("the editor opened no window"))?;
    let scene = app
        .document()
        .and_then(Document::scene)
        .ok_or_else(|| anyhow!("{} has no scene the editor can show", file.display()))?;
    let timing = scene.timing();
    let frames = timing.duration.get();
    let fps = u64::from(timing.fps.get().max(1));
    let mut counts = Counts::of(scene, frames);
    app.dispatch(Intent::SetDetail(detail), main, &mut files);
    app.dispatch(Intent::TogglePlay, main, &mut files);
    app.frame(main, &Input::default(), window, &mut files);
    let calls = frames * rounds.max(1);
    let times = (1..=u64::from(calls))
        .map(|call| {
            let input = Input {
                time_ms: (call * 1000).div_ceil(fps),
                ..Input::default()
            };
            let start = Instant::now();
            app.frame(main, &input, window, &mut files);
            start.elapsed()
        })
        .collect();
    counts.size = format!("{width}x{height}");
    counts.frames = calls;
    Ok((counts, summarize(times)))
}
