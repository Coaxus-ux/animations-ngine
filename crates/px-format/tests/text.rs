#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use px_core::Frame;
use px_engine::{FrameBuffer, Scene};
use px_format::{LoadOptions, Loaded, load_scene};

const HEAD: &str = r##"palette "p" { ramp "sky" "#000000" "#111111"; ramp "ink" "#ffffff" }
font "blocky" height=3 spacing=1 {
    glyph "I" """
        #
        #
        #
        """
    glyph "O" """
        ###
        #.#
        ###
        """
    glyph " " width=2
}"##;

fn source(body: &str) -> String {
    format!(
        "{HEAD}\nscene \"main\" palette=\"p\" width=24 height=8 fps=4 duration=\"2s\" background=\"sky\" {{\n    param title \"OI\"\n{body}\n}}\n"
    )
}

fn load(body: &str) -> Loaded {
    load_scene(&source(body), LoadOptions::default()).unwrap()
}

fn error(body: &str) -> String {
    load_scene(&source(body), LoadOptions::default())
        .unwrap_err()
        .diagnostics[0]
        .message
        .clone()
}

/// Rows of the frame as text: `#` where the ink paint is.
fn picture(scene: &Scene, frame: u32) -> Vec<String> {
    let mut buffer = FrameBuffer::new(scene);
    let view = buffer.render(scene, Frame::new(frame)).unwrap();
    let ink = scene.shapes()[0].paint;
    view.cells()
        .chunks(usize::from(scene.size().width()))
        .map(|row| {
            row.iter()
                .map(|c| if *c == ink { '#' } else { '.' })
                .collect()
        })
        .collect()
}

#[test]
fn text_is_drawn_glyph_by_glyph_with_spacing() {
    let scene = load("text \"OI O\" font=\"blocky\" x=1 y=1 paint=\"ink\"").scene;
    let rows = picture(&scene, 0);
    assert_eq!(rows[1], ".###.#....###...........");
    assert_eq!(rows[2], ".#.#.#....#.#...........");
    assert_eq!(rows[3], ".###.#....###...........");
}

#[test]
fn align_places_the_anchor_at_the_center_or_right() {
    let centered =
        load("text \"OI\" font=\"blocky\" x=12 y=0 align=\"center\" paint=\"ink\"").scene;
    assert_eq!(picture(&centered, 0)[0], "..........###.#.........");
    let right = load("text \"OI\" font=\"blocky\" x=24 y=0 align=\"right\" paint=\"ink\"").scene;
    assert_eq!(picture(&right, 0)[0], "...................###.#");
}

#[test]
fn reveal_shows_characters_in_order_and_new_lines_stack() {
    let scene =
        load("text \"O\\nI\" font=\"blocky\" x=0 y=0 reveal=\"=t * 2\" paint=\"ink\"").scene;
    let early = picture(&scene, 0);
    assert!(
        early.iter().all(|row| !row.contains('#')),
        "nothing before t = 0.5s"
    );
    let first = picture(&scene, 2);
    assert_eq!(&first[0][..4], "###.");
    assert!(
        !first[5].contains('#'),
        "the second line is not revealed yet"
    );
    let both = picture(&scene, 4);
    assert_eq!(
        &both[5][..2],
        "#.",
        "the second line starts line-height (5) lower"
    );
}

#[test]
fn size_and_zoom_keep_glyph_pixels_touching() {
    let scene =
        load("camera zoom=1.5\ntext \"O\" font=\"blocky\" x=9 y=1 size=2 paint=\"ink\"").scene;
    let rows = picture(&scene, 0);
    let inked: Vec<&String> = rows.iter().filter(|row| row.contains('#')).collect();
    let top = inked[0].trim_matches('.');
    assert!(
        top.len() >= 8 && !top.contains('.'),
        "top edge is solid: {top}"
    );
}

#[test]
fn scatter_moves_pixels_and_zero_puts_them_back() {
    let scattered =
        load("text \"O\" font=\"blocky\" x=10 y=2 scatter=\"=4 - t * 2\" paint=\"ink\"").scene;
    assert_ne!(picture(&scattered, 0), picture(&scattered, 8));
    let still = load("text \"O\" font=\"blocky\" x=10 y=2 paint=\"ink\"").scene;
    assert_eq!(picture(&scattered, 8), picture(&still, 0));
}

#[test]
fn text_params_and_missing_glyphs() {
    let loaded = load("text \"$title\" font=\"blocky\" x=0 y=0 paint=\"ink\"");
    assert_eq!(&picture(&loaded.scene, 0)[0][..5], "###.#");
    let boxed = load("text \"OZ\" font=\"blocky\" x=0 y=0 paint=\"ink\"");
    assert!(boxed.warnings[0].message.contains("no glyph for \"Z\""));
    assert_eq!(
        &picture(&boxed.scene, 0)[1][..7],
        "#.#.#.#",
        "Z is a hollow box"
    );
}

#[test]
fn text_mistakes_are_explained() {
    assert!(
        error("text \"O\" font=\"blocki\" x=0 y=0 paint=\"ink\"")
            .contains("Did you mean `blocky`?")
    );
    assert!(
        error("text \"O\" font=\"blocky\" x=0 y=0 align=\"middle\" paint=\"ink\"")
            .contains("left, center or right")
    );
    assert!(error("text \"$nope\" font=\"blocky\" x=0 y=0 paint=\"ink\"").contains("nope"));
}

#[test]
fn bold_thickens_each_glyph_one_pixel_right() {
    let scene = load("text \"I[b]I[/b]I\" font=\"blocky\" x=0 y=0 paint=\"ink\"").scene;
    assert_eq!(&picture(&scene, 0)[0][..7], "#.##.#.");
    let whole = load("text \"I\" font=\"blocky\" x=0 y=0 bold=#true paint=\"ink\"").scene;
    assert_eq!(&picture(&whole, 0)[2][..3], "##.");
}

#[test]
fn a_bigger_size_sits_on_the_same_bottom() {
    let scene = load("text \"O[size=2]O[/size]\" font=\"blocky\" x=0 y=0 paint=\"ink\"").scene;
    let rows = picture(&scene, 0);
    assert_eq!(&rows[0][..10], "....######");
    assert_eq!(&rows[3][..10], "###.##..##");
    assert_eq!(&rows[5][..10], "###.######");
}

/// The built-in font needs no `use` and no `font`; italic leans its rows 0–3 one pixel.
#[test]
fn italic_leans_the_built_in_font() {
    let scene = |italic: &str| {
        let source = format!(
            "{}\nscene \"s\" palette=\"p\" width=8 height=9 fps=4 duration=\"1s\" background=\"sky\" {{\n    text \"|\" x=0 y=0 {italic} paint=\"ink\"\n}}",
            r##"palette "p" { ramp "sky" "#000000"; ramp "ink" "#ffffff" }"##
        );
        load_scene(&source, LoadOptions::default()).unwrap().scene
    };
    let column = |rows: &[String], row: usize| rows[row].find('#').unwrap();
    let upright = picture(&scene(""), 0);
    let leaning = picture(&scene("italic=#true"), 0);
    for (row, lean) in [(0, 1), (3, 1), (4, 0), (8, 0)] {
        assert_eq!(
            column(&leaning, row),
            column(&upright, row) + lean,
            "row {row}"
        );
    }
}

#[test]
fn text_needs_its_words_first() {
    assert!(error("text x=0 y=0 paint=\"ink\"").contains("needs its words first"));
}

#[test]
fn glyph_grids_are_checked() {
    let bad = format!(
        "{}\n{}",
        r##"palette "p" { ramp "sky" "#000000" }"##,
        "font \"f\" height=2 {\n    glyph \"A\" \"\"\"\n        ##\n        #\n        \"\"\"\n}\nscene \"s\" palette=\"p\" width=4 height=4 duration=\"1s\" background=\"sky\" {}"
    );
    let messages: Vec<String> = load_scene(&bad, LoadOptions::default())
        .unwrap_err()
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("same width")),
        "{messages:?}"
    );
}
