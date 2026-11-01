#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use std::io::Cursor;

use px_core::Rgb;
use px_export::{
    CellRect, ExportError, JsonWriter, Padding, RgbaImage, Scale, SheetLayout, SheetMeta,
    aseprite_json, compose, place, write_png_rgba,
};

fn solid(width: u32, height: u32, color: Rgb) -> RgbaImage {
    let mut image = RgbaImage::new(width, height).unwrap();
    image.pixels.fill(Some(color));
    image
}

fn decode(png: &[u8]) -> (png::OutputInfo, Vec<u8>) {
    let mut reader = png::Decoder::new(Cursor::new(png)).read_info().unwrap();
    let mut buffer = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buffer).unwrap();
    buffer.truncate(info.buffer_size());
    (info, buffer)
}

#[test]
fn rows_layout_places_cells_with_border_and_shape_padding() {
    let padding = Padding {
        border: 2,
        shape: 1,
    };
    let (size, cells) = place(5, (4, 3), SheetLayout::Rows { columns: 2 }, padding).unwrap();
    assert_eq!(size, (2 + 4 + 1 + 4 + 2, 2 + 3 + 1 + 3 + 1 + 3 + 2));
    let at = |x, y| CellRect { x, y, w: 4, h: 3 };
    assert_eq!(cells, [at(2, 2), at(7, 2), at(2, 6), at(7, 6), at(2, 10)]);
}

#[test]
fn columns_layout_fills_top_to_bottom_first() {
    let (size, cells) = place(
        3,
        (2, 2),
        SheetLayout::Columns { rows: 2 },
        Padding::default(),
    )
    .unwrap();
    assert_eq!(size, (4, 4));
    assert_eq!([cells[0].y, cells[1].y, cells[2].x], [0, 2, 2]);
}

#[test]
fn zero_columns_or_rows_is_a_sheet_error() {
    let zero_columns = place(
        1,
        (1, 1),
        SheetLayout::Rows { columns: 0 },
        Padding::default(),
    );
    let zero_rows = place(
        1,
        (1, 1),
        SheetLayout::Columns { rows: 0 },
        Padding::default(),
    );
    assert!(matches!(zero_columns, Err(ExportError::Sheet(_))));
    assert!(matches!(zero_rows, Err(ExportError::Sheet(_))));
    assert!(matches!(
        compose(
            &[solid(1, 1, Rgb::default()), solid(2, 1, Rgb::default())],
            SheetLayout::HorizontalStrip,
            Padding::default()
        ),
        Err(ExportError::Sheet(_))
    ));
}

#[test]
fn a_strip_wider_than_the_canvas_limit_still_composes() {
    let frames: Vec<_> = (0..64).map(|i| solid(64, 64, Rgb::new(i, 0, 0))).collect();
    let (sheet, cells) =
        compose(&frames, SheetLayout::HorizontalStrip, Padding::default()).unwrap();
    assert_eq!((sheet.width, sheet.height, cells.len()), (4096, 64, 64));
    assert_eq!(sheet.pixels[63 * 64], Some(Rgb::new(63, 0, 0)));
    let mut png = Vec::new();
    write_png_rgba(&mut png, &sheet, Scale::new(1).unwrap()).unwrap();
    let (info, _) = decode(&png);
    assert_eq!((info.width, info.height), (4096, 64));
}

#[test]
fn rgba_png_decodes_with_alpha_zero_for_none() {
    let mut image = RgbaImage::new(2, 1).unwrap();
    image.pixels[0] = Some(Rgb::new(1, 2, 3));
    let mut png = Vec::new();
    write_png_rgba(&mut png, &image, Scale::new(2).unwrap()).unwrap();
    let (info, pixels) = decode(&png);
    assert_eq!(
        (info.width, info.height, info.color_type),
        (4, 2, png::ColorType::Rgba)
    );
    let row = [1, 2, 3, 255, 1, 2, 3, 255, 0, 0, 0, 0, 0, 0, 0, 0];
    assert_eq!(pixels, [row, row].concat());
}

#[test]
fn json_writer_escapes_quotes_and_control_characters() {
    let mut json = JsonWriter::new();
    json.begin_object();
    json.key("a\"b");
    json.string("back\\slash\n\t\r\u{1}");
    json.key("n");
    json.begin_array();
    json.int(-1);
    json.bool(true);
    json.begin_object();
    json.end_object();
    json.end_array();
    json.end_object();
    assert_eq!(
        json.finish(),
        r#"{"a\"b":"back\\slash\n\t\r\u0001","n":[-1,true,{}]}"#
    );
}

#[test]
fn aseprite_json_matches_the_golden_for_four_frames() {
    let (size, cells) = place(
        4,
        (16, 16),
        SheetLayout::HorizontalStrip,
        Padding::default(),
    )
    .unwrap();
    let meta = SheetMeta {
        title: "walk",
        image: "walk.png",
        size,
        scale: 2,
        frame: (16, 16),
        cells: &cells,
        duration_ms: 100,
        layers: &["body"],
    };
    let expected = concat!(
        r#"{"frames":{"#,
        r#""walk 0.kdl":{"frame":{"x":0,"y":0,"w":32,"h":32},"rotated":false,"trimmed":false,"spriteSourceSize":{"x":0,"y":0,"w":32,"h":32},"sourceSize":{"w":32,"h":32},"duration":100},"#,
        r#""walk 1.kdl":{"frame":{"x":32,"y":0,"w":32,"h":32},"rotated":false,"trimmed":false,"spriteSourceSize":{"x":0,"y":0,"w":32,"h":32},"sourceSize":{"w":32,"h":32},"duration":100},"#,
        r#""walk 2.kdl":{"frame":{"x":64,"y":0,"w":32,"h":32},"rotated":false,"trimmed":false,"spriteSourceSize":{"x":0,"y":0,"w":32,"h":32},"sourceSize":{"w":32,"h":32},"duration":100},"#,
        r#""walk 3.kdl":{"frame":{"x":96,"y":0,"w":32,"h":32},"rotated":false,"trimmed":false,"spriteSourceSize":{"x":0,"y":0,"w":32,"h":32},"sourceSize":{"w":32,"h":32},"duration":100}},"#,
        r#""meta":{"app":"https://github.com/px/pxe","version":""#,
        env!("CARGO_PKG_VERSION"),
        r#"","image":"walk.png","format":"RGBA8888","size":{"w":128,"h":32},"scale":"2","frameTags":[],"layers":[{"name":"body","opacity":255,"blendMode":"normal"}],"slices":[]}}"#,
    );
    assert_eq!(aseprite_json(&meta), expected);
}
