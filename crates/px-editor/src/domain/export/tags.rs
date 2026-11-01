use px_format::Marker;

use crate::domain::timeline::frame_at;

pub type Tag = (String, u32, u32);

#[must_use]
pub fn frame_tags(markers: &[Marker], fps: u16, frame_count: u32) -> Vec<Tag> {
    let Some(last) = frame_count.checked_sub(1) else {
        return Vec::new();
    };
    let mut sorted: Vec<&Marker> = markers.iter().collect();
    sorted.sort_by(|a, b| a.at.total_cmp(&b.at));
    let starts: Vec<u32> = sorted
        .iter()
        .map(|marker| frame_at(marker.at, fps, frame_count))
        .collect();
    sorted
        .iter()
        .zip(&starts)
        .enumerate()
        .filter_map(|(index, (marker, from))| {
            let to = match starts.get(index + 1) {
                Some(next) if next <= from => return None,
                Some(next) => next - 1,
                None => last,
            };
            Some((marker.name.clone(), *from, to))
        })
        .collect()
}
