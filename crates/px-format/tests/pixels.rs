#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use px_core::Frame;
use px_engine::{FrameBuffer, Scene};
use px_format::{LoadOptions, load_scene};

const HEAD: &str =
    r##"palette "p" { ramp "sky" "#000000" "#111111"; ramp "ink" "#ff0000" "#00ff00" }"##;

fn source(body: &str) -> String {
    format!(
        "{HEAD}\nscene \"main\" palette=\"p\" width=6 height=4 fps=4 duration=\"2s\" background=\"sky.0\" {{\n{body}\n}}\n"
    )
}

fn load(body: &str) -> Scene {
    load_scene(&source(body), LoadOptions::default())
        .unwrap()
        .scene
}

fn error(body: &str) -> String {
    load_scene(&source(body), LoadOptions::default())
        .unwrap_err()
        .diagnostics[0]
        .message
        .clone()
}

/// Rows of the frame: `.` for the background, `0`/`1` for the two ink tones.
fn picture(scene: &Scene, frame: u32) -> Vec<String> {
    let mut buffer = FrameBuffer::new(scene);
    let view = buffer.render(scene, Frame::new(frame)).unwrap();
    let background = scene.background();
    view.cells()
        .chunks(usize::from(scene.size().width()))
        .map(|row| {
            row.iter()
                .map(|cell| match cell {
                    cell if *cell == background => '.',
                    cell => char::from(b'0' + cell.tone.index()),
                })
                .collect()
        })
        .collect()
}

#[test]
fn each_pixel_keeps_its_own_paint() {
    let scene = load(
        r#"pixels "art" x=1 y=1 {
            legend { a "ink.0"; b "ink.1" }
            cel """
            ab.
            .ba
            """
        }"#,
    );
    assert_eq!(picture(&scene, 0), ["......", ".01...", "..10..", "......"]);
}

#[test]
fn a_cel_shows_from_its_frame_until_the_next() {
    let scene = load(
        r#"pixels "art" x=0 y=0 {
            legend { a "ink.0"; b "ink.1" }
            cel frame=0 "a"
            cel frame=4 x=2 "b"
        }"#,
    );
    assert_eq!(picture(&scene, 3)[0], "0.....");
    assert_eq!(picture(&scene, 4)[0], "..1...");
    assert_eq!(picture(&scene, 7)[0], "..1...");
}

#[test]
fn two_character_codes_and_empty_pixels() {
    let scene = load(
        r#"pixels "art" x=0 y=0 {
            legend { aa "ink.0"; bb "ink.1" }
            cel "aa..bb"
        }"#,
    );
    assert_eq!(picture(&scene, 0)[0], "0.1...");
}

#[test]
fn mistakes_are_reported() {
    let grid = |legend: &str, cel: &str| {
        error(&format!(
            "pixels \"art\" x=0 y=0 {{\n legend {{ {legend} }}\n cel \"\"\"\n{cel}\n\"\"\"\n}}"
        ))
    };
    assert!(grid("a \"ink.0\"", "ab").contains("code `b` is not in the legend"));
    assert!(grid("a \"ink.0\"", "aa\na").contains("same width"));
    assert!(grid("a \"ink.0\"; bb \"ink.1\"", "a").contains("same length"));
    assert!(grid(". \"ink.0\"", "a").contains("other than `.`"));
    let twice = error(r#"pixels "art" x=0 y=0 { legend { a "ink.0" }; cel "a"; cel frame=0 "a" }"#);
    assert!(twice.contains("two cels start at frame 0"), "{twice}");
}

#[test]
fn written_pixels_load_back_to_the_same_frames() {
    use px_core::hash_cells;
    use px_engine::Geometry;
    use px_format::edit::{NodePath, Source};
    use px_format::write_pixels;

    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/pixel-room.kdl");
    let text = std::fs::read_to_string(path).unwrap();
    let loaded = load_scene(&text, LoadOptions::default()).unwrap();
    let mut source = Source::parse(text.clone()).unwrap();
    let scene_path = NodePath::root().child("scene", Some("pixel-room"), 0);
    let targets: Vec<(NodePath, usize)> = [("cat", "cat-art"), ("plant", "plant-art")]
        .into_iter()
        .map(|(layer, art)| {
            let node = scene_path
                .child("layer", Some(layer), 0)
                .child("pixels", Some(art), 0);
            let (start, _) = source.span(&node).unwrap();
            let shape = loaded
                .origins
                .iter()
                .position(|origin| origin.offset == start)
                .unwrap();
            (node, shape)
        })
        .collect();
    for (node, shape) in targets {
        let Geometry::Pixels(pixels) = &loaded.scene.shapes()[shape].geometry else {
            panic!("{node} is not a pixels shape");
        };
        let children = write_pixels(&pixels.cels, &loaded.paint_names).unwrap();
        source.set_children(&node, &children).unwrap();
    }
    let reloaded = load_scene(source.text(), LoadOptions::default()).unwrap();
    let hashes = |scene: &Scene| -> Vec<u64> {
        let mut buffer = FrameBuffer::new(scene);
        (0..12)
            .map(|frame| hash_cells(buffer.render(scene, Frame::new(frame)).unwrap().cells()))
            .collect()
    };
    assert_eq!(hashes(&loaded.scene), hashes(&reloaded.scene));
    assert!(
        source.text().contains("legend { a \"fur.0\"; b \"fur.1\";"),
        "{}",
        source.text()
    );
}
