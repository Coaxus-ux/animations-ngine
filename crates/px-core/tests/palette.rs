#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use px_core::{MAX_RAMPS, MAX_TONES, Paint, Palette, PaletteError, Rgb, Tone, ToneName};

fn grays(count: u8) -> Vec<Rgb> {
    (0..count)
        .map(|value| Rgb::new(value, value, value))
        .collect()
}

#[test]
fn push_ramp_returns_sequential_ids_and_resolves_colors() {
    let mut palette = Palette::new();
    let first = palette.push_ramp(&grays(3)).unwrap();
    let second = palette.push_ramp(&grays(5)).unwrap();
    assert_eq!((first.index(), second.index()), (0, 1));
    let paint = palette.paint(second, Tone::new(4)).unwrap();
    assert_eq!(palette.resolve(paint), Ok(Rgb::new(4, 4, 4)));
}

#[test]
fn push_ramp_rejects_empty_and_too_long_ramps() {
    let mut palette = Palette::new();
    assert_eq!(palette.push_ramp(&[]), Err(PaletteError::EmptyRamp));
    let too_long = grays(u8::try_from(MAX_TONES + 1).unwrap());
    assert_eq!(
        palette.push_ramp(&too_long),
        Err(PaletteError::RampTooLong {
            len: MAX_TONES + 1,
            max: MAX_TONES
        })
    );
}

#[test]
fn push_ramp_reports_full_at_exact_capacity() {
    let mut palette = Palette::new();
    for _ in 0..MAX_RAMPS {
        palette.push_ramp(&grays(1)).unwrap();
    }
    assert_eq!(
        palette.push_ramp(&grays(1)),
        Err(PaletteError::Full {
            capacity: MAX_RAMPS
        })
    );
}

#[test]
fn paint_rejects_tones_past_the_ramp() {
    let mut palette = Palette::new();
    let ramp = palette.push_ramp(&grays(2)).unwrap();
    let error = palette.paint(ramp, Tone::new(2));
    assert_eq!(
        error,
        Err(PaletteError::ToneOutOfRange {
            ramp,
            tone: Tone::new(2),
            len: 2
        })
    );
    assert!(!palette.contains(Paint {
        ramp,
        tone: Tone::new(2)
    }));
}

#[test]
fn named_tones_match_the_standard_five_tone_ramp() {
    let resolved: Vec<u8> = [
        ToneName::Deep,
        ToneName::Shade,
        ToneName::Base,
        ToneName::Lit,
        ToneName::Hi,
    ]
    .iter()
    .map(|name| name.resolve(5).unwrap().index())
    .collect();
    assert_eq!(resolved, [0, 1, 2, 3, 4]);
}

#[test]
fn named_tones_stay_inside_short_and_long_ramps() {
    for len in 1..=u8::try_from(MAX_TONES).unwrap() {
        for name in [
            ToneName::Deep,
            ToneName::Shade,
            ToneName::Base,
            ToneName::Lit,
            ToneName::Hi,
        ] {
            assert!(
                name.resolve(len).unwrap().index() < len,
                "{name:?} on {len} tones"
            );
        }
    }
    assert_eq!(ToneName::Base.resolve(0), None);
}

#[test]
fn parse_hex_accepts_both_cases_and_reports_errors() {
    assert_eq!(Rgb::parse_hex("#0aFf10"), Ok(Rgb::new(10, 255, 16)));
    assert!(Rgb::parse_hex("0aff10").is_err());
    assert!(Rgb::parse_hex("#0aff1").is_err());
    assert!(Rgb::parse_hex("#0aff1g").is_err());
}
