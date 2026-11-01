use crate::color::Rgb;
use crate::palette::MAX_TONES;

/// A color as hue (degrees, `0..360`), saturation and value (`0..=1`), for pickers and
/// ramp generation. Plain `f64` arithmetic through `libm`, so results match on every OS.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Hsv {
    pub h: f64,
    pub s: f64,
    pub v: f64,
}

impl Hsv {
    #[must_use]
    pub fn new(h: f64, s: f64, v: f64) -> Self {
        Self {
            h: wrap_hue(h),
            s: s.clamp(0.0, 1.0),
            v: v.clamp(0.0, 1.0),
        }
    }

    #[must_use]
    pub fn from_rgb(rgb: Rgb) -> Self {
        let red = f64::from(rgb.r) / 255.0;
        let green = f64::from(rgb.g) / 255.0;
        let blue = f64::from(rgb.b) / 255.0;
        let max = red.max(green).max(blue);
        let min = red.min(green).min(blue);
        let delta = max - min;
        let hue = if delta <= 0.0 {
            0.0
        } else if rgb.r >= rgb.g && rgb.r >= rgb.b {
            60.0 * ((green - blue) / delta)
        } else if rgb.g >= rgb.b {
            60.0 * ((blue - red) / delta + 2.0)
        } else {
            60.0 * ((red - green) / delta + 4.0)
        };
        let saturation = if max <= 0.0 { 0.0 } else { delta / max };
        Self::new(hue, saturation, max)
    }

    #[must_use]
    pub fn to_rgb(self) -> Rgb {
        let chroma = self.v * self.s;
        let sector = self.h / 60.0;
        let second = chroma * (1.0 - libm::fabs(libm::fmod(sector, 2.0) - 1.0));
        let floor = self.v - chroma;
        let (red, green, blue) = match sector_index(sector) {
            0 => (chroma, second, 0.0),
            1 => (second, chroma, 0.0),
            2 => (0.0, chroma, second),
            3 => (0.0, second, chroma),
            4 => (second, 0.0, chroma),
            _ => (chroma, 0.0, second),
        };
        Rgb::new(
            channel(red + floor),
            channel(green + floor),
            channel(blue + floor),
        )
    }

    #[must_use]
    pub fn shifted(self, hue: f64, saturation: f64, value: f64) -> Self {
        Self::new(self.h + hue, self.s + saturation, self.v + value)
    }
}

/// A ramp of `steps` tones from dark to light around `base`, the pixel-art way: the value
/// spreads by `contrast`, dark tones turn their hue by `-hue_shift` (toward cool colors)
/// and light tones by `+hue_shift` (toward warm ones), and saturation falls by
/// `saturation` toward the light end.
#[must_use]
pub fn ramp(
    base: Rgb,
    steps: u8,
    hue_shift: f64,
    contrast: f64,
    saturation: f64,
) -> [Rgb; MAX_TONES] {
    let steps = steps.clamp(1, u8::try_from(MAX_TONES).unwrap_or(u8::MAX));
    let center = Hsv::from_rgb(base);
    let mut tones = [base; MAX_TONES];
    for (index, tone) in tones.iter_mut().enumerate().take(usize::from(steps)) {
        let position = if steps == 1 {
            0.0
        } else {
            f64::from(u8::try_from(index).unwrap_or(u8::MAX)) / f64::from(steps - 1) * 2.0 - 1.0
        };
        *tone = center
            .shifted(
                position * hue_shift,
                -position.max(0.0) * saturation,
                position * contrast,
            )
            .to_rgb();
    }
    tones
}

fn wrap_hue(h: f64) -> f64 {
    let wrapped = libm::fmod(h, 360.0);
    if wrapped < 0.0 {
        wrapped + 360.0
    } else {
        wrapped
    }
}

/// Which of the six hue sectors `sector` (0..6) falls in.
fn sector_index(sector: f64) -> u8 {
    let mut index = 0;
    while index < 5 && f64::from(index + 1) <= sector {
        index += 1;
    }
    index
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped to 0..=255 before the cast"
)]
fn channel(value: f64) -> u8 {
    libm::floor(value.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsv_round_trips_a_lattice_of_rgb_triples_exactly() {
        for r in (0u8..=255).step_by(15) {
            for g in (0u8..=255).step_by(15) {
                for b in (0u8..=255).step_by(15) {
                    let rgb = Rgb::new(r, g, b);
                    assert_eq!(Hsv::from_rgb(rgb).to_rgb(), rgb);
                }
            }
        }
    }

    #[test]
    fn pure_colors_have_the_expected_hue() {
        assert_eq!(Hsv::from_rgb(Rgb::new(255, 0, 0)).h, 0.0);
        assert_eq!(Hsv::from_rgb(Rgb::new(0, 255, 0)).h, 120.0);
        assert_eq!(Hsv::from_rgb(Rgb::new(0, 0, 255)).h, 240.0);
        assert_eq!(
            Hsv::new(-30.0, 2.0, 0.5),
            Hsv {
                h: 330.0,
                s: 1.0,
                v: 0.5
            }
        );
    }

    #[test]
    fn a_generated_ramp_has_the_requested_steps_and_is_monotonic_in_value() {
        let tones = ramp(Rgb::new(180, 90, 60), 5, 20.0, 0.35, 0.3);
        let values = tones.map(|tone| Hsv::from_rgb(tone).v);
        let values = &values[..5];
        assert!(
            values.windows(2).all(|pair| pair[0] < pair[1]),
            "{values:?}"
        );
        assert_eq!(tones[2], Rgb::new(180, 90, 60));
        assert_eq!(
            ramp(Rgb::new(1, 2, 3), 1, 0.0, 0.0, 0.0)[0],
            Rgb::new(1, 2, 3)
        );
    }
}
