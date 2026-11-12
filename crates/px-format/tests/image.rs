#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use std::path::Path;

use px_core::{Frame, Paint};
use px_engine::FrameBuffer;
use px_format::{LoadOptions, MemorySources, load};

const PALETTE: &str = r#"palette "p" { ramp "sky" "000000 101010"; ramp "wood" "604020 a07040" }"#;
const CLEAR: [u8; 4] = [0, 0, 0, 0];
const DARK_WOOD: [u8; 4] = [0x60, 0x40, 0x20, 255];
const LIGHT_WOOD: [u8; 4] = [0xa0, 0x70, 0x40, 255];

fn png(width: u32, height: u32, pixels: &[[u8; 4]]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&pixels.concat()).unwrap();
    writer.finish().unwrap();
    bytes
}

fn art() -> Vec<u8> {
    png(2, 2, &[DARK_WOOD, CLEAR, LIGHT_WOOD, DARK_WOOD])
}

fn scene(palette: &str, body: &str) -> String {
    format!(
        "{palette}\nscene \"main\" palette=\"p\" width=4 height=3 fps=1 duration=\"1f\" background=\"sky\" {{\n{body}\n}}\n"
    )
}

fn sources(palette: &str, body: &str, image: Vec<u8>) -> MemorySources {
    MemorySources::default()
        .with("scenes/main.kdl", scene(palette, body))
        .with_bytes("scenes/art/wood.png", image)
}

fn cells(body: &str, image: Vec<u8>) -> Vec<Paint> {
    let mut sources = sources(PALETTE, body, image);
    let scene = load(
        Path::new("scenes/main.kdl"),
        &mut sources,
        LoadOptions::default(),
    )
    .unwrap()
    .scene;
    let mut buffer = FrameBuffer::new(&scene);
    buffer
        .render(&scene, Frame::new(0))
        .unwrap()
        .cells()
        .to_vec()
}

fn errors_with(palette: &str, body: &str, image: Vec<u8>) -> Vec<String> {
    let mut sources = sources(palette, body, image);
    load(
        Path::new("scenes/main.kdl"),
        &mut sources,
        LoadOptions::default(),
    )
    .unwrap_err()
    .diagnostics
    .iter()
    .map(|diagnostic| diagnostic.message.clone())
    .collect()
}

fn errors(body: &str, image: Vec<u8>) -> Vec<String> {
    errors_with(PALETTE, body, image)
}

fn mentions(found: &[String], words: &str) -> bool {
    found.iter().any(|message| message.contains(words))
}

#[test]
fn an_image_paints_the_same_cells_as_the_shapes_it_was_baked_from() {
    let shapes = "rect 1 1 1 1 paint=wood.0\nrect 1 2 1 1 paint=wood.1\nrect 2 2 1 1 paint=wood.0";
    let image = r#"pixels "art" src="art/wood.png" x=1 y=1"#;
    assert_eq!(cells(image, art()), cells(shapes, art()));
}

#[test]
fn the_image_files_are_listed_as_assets() {
    let mut sources = sources(PALETTE, r#"pixels src="art/wood.png""#, art());
    let loaded = load(
        Path::new("scenes/main.kdl"),
        &mut sources,
        LoadOptions::default(),
    )
    .unwrap();
    assert_eq!(loaded.assets, [Path::new("scenes/art/wood.png")]);
}

#[test]
fn a_color_outside_the_palette_is_an_error() {
    let image = png(1, 1, &[[1, 2, 3, 255]]);
    let found = errors(r#"pixels src="art/wood.png""#, image);
    assert!(
        mentions(&found, "#010203 is not in the palette"),
        "{found:?}"
    );
}

#[test]
fn a_color_in_two_ramps_needs_the_legend_to_choose() {
    let twice = r#"palette "p" { ramp "sky" "000000 101010"; ramp "wood" "604020 a07040"; ramp "bark" "604020" }"#;
    let found = errors_with(twice, r#"pixels src="art/wood.png""#, art());
    assert!(mentions(&found, "is both bark.0 and wood.0"), "{found:?}");
    let chosen = r##"pixels src="art/wood.png" { legend { "#604020" wood.0 } }"##;
    let mut sources = sources(twice, chosen, art());
    assert!(
        load(
            Path::new("scenes/main.kdl"),
            &mut sources,
            LoadOptions::default()
        )
        .is_ok()
    );
}

#[test]
fn image_mistakes_are_reported() {
    let source = r#"pixels src="art/wood.png""#;
    assert!(mentions(
        &errors(r#"pixels src="art/none.png""#, art()),
        "src \"art/none.png\""
    ));
    assert!(mentions(
        &errors(r#"pixels src="art/wood.png" { cel "a" }"#, art()),
        "not both"
    ));
    assert!(mentions(
        &errors(source, png(1, 1, &[[0x60, 0x40, 0x20, 9]])),
        "partly transparent"
    ));
    assert!(mentions(
        &errors(source, png(2049, 1, &vec![DARK_WOOD; 2049])),
        "at most"
    ));
    assert!(mentions(
        &errors(source, b"not a png".to_vec()),
        "src \"art/wood.png\""
    ));
    assert!(mentions(
        &errors(
            r#"pixels src="art/wood.png" { legend { a wood.0 } }"#,
            art()
        ),
        "legend keys are colors"
    ));
}
