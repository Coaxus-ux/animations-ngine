#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use px_core::{CanvasView, FrameRate, Paint, Palette, Rgb, Size, Tone};
use px_export::{ExportError, GifWriter, Scale, indexed_len, resolve_rgb, write_png};

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
