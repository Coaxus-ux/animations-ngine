use px_format::edit::NodePath;

use super::Document;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkerAt {
    pub name: String,
    pub frame: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyRow {
    pub label: String,
    pub path: Option<NodePath>,
    pub frames: Vec<u32>,
}

#[must_use]
pub fn frame_at(seconds: f64, fps: u16, frame_count: u32) -> u32 {
    let last = f64::from(frame_count.saturating_sub(1));
    let frame = px_core::floor(seconds * f64::from(fps) + 0.5).clamp(0.0, last);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0..=last, which is a u32"
    )]
    let frame = frame as u32;
    frame
}

#[must_use]
pub fn markers(document: &Document) -> Vec<MarkerAt> {
    let (Some(loaded), Some((fps, frames))) = (document.loaded(), timing(document)) else {
        return Vec::new();
    };
    loaded
        .timeline
        .markers
        .iter()
        .map(|marker| MarkerAt {
            name: marker.name.clone(),
            frame: frame_at(marker.at, fps, frames),
        })
        .collect()
}

#[must_use]
pub fn key_rows(document: &Document) -> Vec<KeyRow> {
    let (Some(loaded), Some((fps, frames))) = (document.loaded(), timing(document)) else {
        return Vec::new();
    };
    let mut rows: Vec<(px_format::Origin, KeyRow)> = Vec::new();
    for keyed in &loaded.timeline.keys {
        let index = rows
            .iter()
            .position(|(origin, _)| *origin == keyed.origin)
            .unwrap_or_else(|| {
                rows.push((
                    keyed.origin,
                    KeyRow {
                        label: keyed.object.clone(),
                        path: (keyed.origin.file == 0)
                            .then(|| document.source().path_at(keyed.origin.offset))
                            .flatten(),
                        frames: Vec::new(),
                    },
                ));
                rows.len() - 1
            });
        let row = &mut rows[index].1;
        row.frames.extend(
            keyed
                .times
                .iter()
                .map(|seconds| frame_at(*seconds, fps, frames)),
        );
    }
    rows.into_iter()
        .map(|(_, mut row)| {
            row.frames.sort_unstable();
            row.frames.dedup();
            row
        })
        .collect()
}

fn timing(document: &Document) -> Option<(u16, u32)> {
    let timing = document.scene()?.timing();
    Some((timing.fps.get(), timing.duration.get().max(1)))
}
