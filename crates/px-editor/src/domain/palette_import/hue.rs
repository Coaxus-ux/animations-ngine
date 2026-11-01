use px_core::{Hsv, Rgb};

const GRAY_SATURATION: f64 = 0.12;
const GRAY: &str = "gray";

const FAMILIES: [(&str, f64, f64); 8] = [
    ("red", 345.0, 15.0),
    ("orange", 15.0, 45.0),
    ("yellow", 45.0, 70.0),
    ("green", 70.0, 165.0),
    ("cyan", 165.0, 200.0),
    ("blue", 200.0, 260.0),
    ("purple", 260.0, 300.0),
    ("magenta", 300.0, 345.0),
];

#[must_use]
pub fn ramps_by_hue(colors: &[Rgb], max_tones: usize) -> Vec<(String, Vec<Rgb>)> {
    let mut families: Vec<Vec<Rgb>> = vec![Vec::new(); FAMILIES.len() + 1];
    for &color in colors {
        families[family_of(Hsv::from_rgb(color))].push(color);
    }
    let names = FAMILIES.iter().map(|(name, _, _)| *name).chain([GRAY]);
    names
        .zip(families)
        .filter(|(_, colors)| !colors.is_empty())
        .flat_map(|(name, mut colors)| {
            colors.sort_by_key(|&color| darkness(color));
            colors
                .chunks(max_tones.max(1))
                .map(|chunk| (name.to_owned(), chunk.to_vec()))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn family_of(hsv: Hsv) -> usize {
    if hsv.s < GRAY_SATURATION {
        return FAMILIES.len();
    }
    FAMILIES
        .iter()
        .position(|&(_, from, to)| {
            if from > to {
                hsv.h >= from || hsv.h < to
            } else {
                hsv.h >= from && hsv.h < to
            }
        })
        .unwrap_or(FAMILIES.len())
}

fn darkness(color: Rgb) -> (u8, u32) {
    (color.r.max(color.g).max(color.b), color.luma())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DARK_RED: Rgb = Rgb::new(0x60, 0x10, 0x10);
    const RED: Rgb = Rgb::new(0xc0, 0x20, 0x20);
    const LIGHT_RED: Rgb = Rgb::new(0xff, 0x80, 0x80);
    const BLUE: Rgb = Rgb::new(0x20, 0x40, 0xc0);
    const YELLOW: Rgb = Rgb::new(0xe0, 0xd0, 0x30);

    fn names(ramps: &[(String, Vec<Rgb>)]) -> Vec<&str> {
        ramps.iter().map(|(name, _)| name.as_str()).collect()
    }

    #[test]
    fn colors_cluster_into_named_hue_ramps_sorted_dark_to_light() {
        let ramps = ramps_by_hue(&[BLUE, LIGHT_RED, YELLOW, DARK_RED, RED], 8);
        assert_eq!(names(&ramps), ["red", "yellow", "blue"]);
        assert_eq!(ramps[0].1, vec![DARK_RED, RED, LIGHT_RED]);
        assert_eq!(ramps[2].1, vec![BLUE]);
    }

    #[test]
    fn grays_go_to_their_own_ramp() {
        let white = Rgb::new(0xff, 0xff, 0xff);
        let black = Rgb::new(0, 0, 0);
        let warm_gray = Rgb::new(0x88, 0x80, 0x7c);
        let ramps = ramps_by_hue(&[white, RED, warm_gray, black], 8);
        assert_eq!(names(&ramps), ["red", "gray"]);
        assert_eq!(ramps[1].1, vec![black, warm_gray, white]);
    }

    #[test]
    fn a_big_family_splits_into_chunks_of_the_same_name() {
        let greens: Vec<Rgb> = (0..5u8)
            .map(|n| Rgb::new(0x10, 0x40 + n * 0x20, 0x10))
            .collect();
        let ramps = ramps_by_hue(&greens, 2);
        assert_eq!(names(&ramps), ["green", "green", "green"]);
        assert_eq!(ramps[2].1, vec![greens[4]]);
    }

    #[test]
    fn red_wraps_around_zero_degrees() {
        let rosy = Hsv::new(350.0, 0.8, 0.8).to_rgb();
        let magenta = Hsv::new(320.0, 0.8, 0.8).to_rgb();
        assert_eq!(
            names(&ramps_by_hue(&[magenta, rosy], 8)),
            ["red", "magenta"]
        );
    }
}
