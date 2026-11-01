use px_format::Marker;

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
        .map(|marker| frame_of(marker.at, fps, last))
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

fn frame_of(seconds: f64, fps: u16, last: u32) -> u32 {
    let frame = px_core::floor(seconds * f64::from(fps) + 0.5);
    let clamped = frame.clamp(0.0, f64::from(last));
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0..=last, which is a u32"
    )]
    let frame = clamped as u32;
    frame
}
