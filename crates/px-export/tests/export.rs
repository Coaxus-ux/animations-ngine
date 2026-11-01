#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use px_core::{CanvasView, FrameRate, Paint, Palette, Rgb, Size, Tone};
use px_export::{ExportError, GifWriter, Scale, indexed_len, resolve_rgb, write_png};

#[test]
fn opaque_gif_bytes_are_unchanged() {
    let mut palette = Palette::new();
    let ramp = palette
        .push_ramp(&[Rgb::new(1, 2, 3), Rgb::new(9, 8, 7), Rgb::new(40, 50, 60)])
        .unwrap();
    let paint = |tone: u8| Paint {
        ramp,
        tone: Tone::new(tone),
    };
    let size = Size::new(4, 3).unwrap();
    let first: Vec<Paint> = (0..12).map(|i| paint(i % 3)).collect();
    let mut second = first.clone();
    second[5] = paint(2);
    second[6] = paint(0);
    let third = second.clone();
    let mut bytes = Vec::new();
    let mut gif = GifWriter::new(
        &mut bytes,
        size,
        &palette,
        Scale::new(2).unwrap(),
        FrameRate::new(12).unwrap(),
    )
    .unwrap();
    for cells in [&first, &second, &third] {
        gif.add_frame(&CanvasView::new(size, cells).unwrap())
            .unwrap();
    }
    drop(gif);
    assert_eq!(bytes.len(), 126);
    assert_eq!(px_core::hash_bytes(&bytes), 0xf0d0_388b_dd03_9bbb);
}

#[test]
fn a_transparent_gif_has_a_transparent_index_and_background_disposal() {
    let mut palette = Palette::new();
    let ink = palette.push_ramp(&[Rgb::new(10, 20, 30)]).unwrap();
    let key = palette.push_ramp(&[Rgb::new(255, 0, 255)]).unwrap();
    let paint = |ramp, tone: u8| Paint {
        ramp,
        tone: Tone::new(tone),
    };
    let size = Size::new(2, 1).unwrap();
    let frames = [
        [paint(ink, 0), paint(key, 0)],
        [paint(key, 0), paint(ink, 0)],
    ];
    let mut bytes = Vec::new();
    let mut gif = GifWriter::with_key(
        &mut bytes,
        size,
        &palette,
        key,
        Scale::new(1).unwrap(),
        FrameRate::new(12).unwrap(),
    )
    .unwrap();
    for cells in &frames {
        gif.add_frame(&CanvasView::new(size, cells).unwrap())
            .unwrap();
    }
    drop(gif);
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = options.read_info(&bytes[..]).unwrap();
    let mut seen = 0;
    while let Some(frame) = decoder.read_next_frame().unwrap() {
        assert_eq!(frame.dispose, gif::DisposalMethod::Background);
        assert_eq!(frame.transparent, Some(1), "the key ramp leaves the table");
        assert_eq!(
            (frame.left, frame.top, frame.width, frame.height),
            (0, 0, 2, 1)
        );
        let alphas: Vec<u8> = frame
            .buffer
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| p[3])
            .collect();
        let expected = if seen == 0 { [255, 0] } else { [0, 255] };
        assert_eq!(alphas, expected);
        assert_eq!(&frame.buffer[seen * 4..seen * 4 + 3], &[10, 20, 30]);
        seen += 1;
    }
    assert_eq!(seen, 2);
}

fn two_pixel_canvas() -> (Palette, [Paint; 2]) {
    let mut palette = Palette::new();
    let ramp = palette
        .push_ramp(&[Rgb::new(1, 2, 3), Rgb::new(9, 8, 7)])
        .unwrap();
    (
        palette,
        [
            Paint {
                ramp,
                tone: Tone::new(0),
            },
            Paint {
                ramp,
                tone: Tone::new(1),
            },
        ],
    )
}

#[test]
fn resolve_upscales_with_nearest_neighbor() {
    let (palette, cells) = two_pixel_canvas();
    let view = CanvasView::new(Size::new(2, 1).unwrap(), &cells).unwrap();
    let scale = Scale::new(2).unwrap();
    let mut rgb = vec![0; indexed_len(&view, scale, 3).unwrap()];
    resolve_rgb(&view, &palette, scale, &mut rgb).unwrap();
    let row = [1, 2, 3, 1, 2, 3, 9, 8, 7, 9, 8, 7];
    assert_eq!(rgb, [row, row].concat());
}

#[test]
fn resolve_rejects_a_wrong_output_length() {
    let (palette, cells) = two_pixel_canvas();
    let view = CanvasView::new(Size::new(2, 1).unwrap(), &cells).unwrap();
    let mut rgb = vec![0; 5];
    let result = resolve_rgb(&view, &palette, Scale::new(1).unwrap(), &mut rgb);
    assert!(matches!(result, Err(ExportError::TooLarge)));
}

#[test]
fn scale_is_bounded() {
    assert!(Scale::new(0).is_none());
    assert!(Scale::new(17).is_none());
}

#[test]
fn png_and_gif_have_valid_signatures() {
    let (palette, cells) = two_pixel_canvas();
    let size = Size::new(2, 1).unwrap();
    let view = CanvasView::new(size, &cells).unwrap();
    let mut png_bytes = Vec::new();
    write_png(&mut png_bytes, &view, &palette, Scale::new(3).unwrap()).unwrap();
    assert!(png_bytes.starts_with(b"\x89PNG\r\n\x1a\n"));

    let mut gif_bytes = Vec::new();
    let mut gif = GifWriter::new(
        &mut gif_bytes,
        size,
        &palette,
        Scale::new(1).unwrap(),
        FrameRate::new(12).unwrap(),
    )
    .unwrap();
    gif.add_frame(&view).unwrap();
    drop(gif);
    assert!(gif_bytes.starts_with(b"GIF89a"));
}

#[test]
fn gif_rejects_palettes_over_256_colors() {
    let mut palette = Palette::new();
    for _ in 0..40 {
        palette.push_ramp(&[Rgb::default(); 7]).unwrap();
    }
    let result = GifWriter::new(
        Vec::new(),
        Size::new(1, 1).unwrap(),
        &palette,
        Scale::new(1).unwrap(),
        FrameRate::new(12).unwrap(),
    );
    assert!(matches!(
        result,
        Err(ExportError::TooManyColors {
            count: 280,
            max: 256
        })
    ));
}

#[test]
fn a_keyed_gif_keeps_one_slot_for_transparency() {
    let keyed = |ramps: usize| {
        let mut palette = Palette::new();
        for _ in 0..ramps {
            palette.push_ramp(&[Rgb::default(); 8]).unwrap();
        }
        let key = palette.push_ramp(&[Rgb::new(255, 0, 255)]).unwrap();
        GifWriter::with_key(
            Vec::new(),
            Size::new(1, 1).unwrap(),
            &palette,
            key,
            Scale::new(1).unwrap(),
            FrameRate::new(12).unwrap(),
        )
        .map(drop)
    };
    assert!(matches!(
        keyed(32),
        Err(ExportError::TooManyColors {
            count: 256,
            max: 255
        })
    ));
    assert!(keyed(31).is_ok());
}

fn decoded_frames(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = options.read_info(bytes).unwrap();
    let width = usize::from(decoder.width());
    let mut screen = vec![0; width * usize::from(decoder.height()) * 3];
    let mut frames = Vec::new();
    while let Some(frame) = decoder.read_next_frame().unwrap() {
        let (left, top, columns) = (
            usize::from(frame.left),
            usize::from(frame.top),
            usize::from(frame.width),
        );
        for (i, pixel) in frame.buffer.as_chunks::<4>().0.iter().enumerate() {
            if pixel[3] != 0 {
                let at = ((top + i / columns) * width + left + i % columns) * 3;
                screen[at..at + 3].copy_from_slice(&pixel[..3]);
            }
        }
        frames.push(screen.clone());
    }
    frames
}

#[test]
fn color_frames_round_trip_through_deltas_and_a_full_table() {
    let gray = |value: u8| Rgb::new(value, value, value);
    let first: Vec<Rgb> = (0..=255).map(|_| gray(0)).collect();
    let all: Vec<Rgb> = (0..=255).map(gray).collect();
    let mut one = all.clone();
    one[200] = Rgb::new(1, 2, 3);
    let frames = [first, all.clone(), all, one];
    let mut bytes = Vec::new();
    let mut gif = GifWriter::with_frame_palettes(
        &mut bytes,
        Size::new(16, 16).unwrap(),
        Scale::new(2).unwrap(),
        FrameRate::new(12).unwrap(),
    )
    .unwrap();
    for frame in &frames {
        gif.add_color_frame(frame).unwrap();
    }
    drop(gif);
    let decoded = decoded_frames(&bytes);
    assert_eq!(decoded.len(), frames.len());
    for (frame, screen) in frames.iter().zip(&decoded) {
        let mut expected = vec![0; 32 * 32 * 3];
        for (i, color) in frame.iter().enumerate() {
            for (dy, dx) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                let at = ((i / 16 * 2 + dy) * 32 + i % 16 * 2 + dx) * 3;
                expected[at..at + 3].copy_from_slice(&[color.r, color.g, color.b]);
            }
        }
        assert_eq!(*screen, expected);
    }
}
