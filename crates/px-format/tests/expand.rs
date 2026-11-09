#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod shorthand;

use std::path::Path;

use px_format::{LoadOptions, MemorySources, load, load_scene};
use shorthand::{PALETTE, assert_error, assert_same, file, hashes, render, scene};

#[test]
fn bare_values_fill_coordinates_in_order() {
    assert_same(
        "",
        "rect 1 2 3 4 paint=ink\nline 0 0 9 9 paint=red\nblob 8 8 circles=\"0,0,3 2,0,2\" paint=ink",
        r#"rect x=1 y=2 width=3 height=4 paint="ink"
line x1=0 y1=0 x2=9 y2=9 paint="red"
blob x=8 y=8 circles="0,0,3 2,0,2" paint="ink""#,
    );
}

#[test]
fn a_leading_plain_string_stays_the_id() {
    assert_same(
        "",
        "rect \"a\" 1 2 3 4 paint=ink\nrect \"=@a.x + 5\" 2 3 4 paint=red",
        r#"rect "a" x=1 y=2 width=3 height=4 paint="ink"
rect x="=@a.x + 5" y=2 width=3 height=4 paint="red""#,
    );
}

#[test]
fn text_and_placed_scenes_keep_their_first_string() {
    assert_same(
        "",
        r#"text "=" 1 2 paint=ink"#,
        r#"text "=" x=1 y=2 paint="ink""#,
    );
    let sugared = format!(
        "{PALETTE}\nscene \"ball\" palette=\"p\" width=4 height=4 fps=4 duration=\"2s\" background=\"sky\" {{ rect 0 0 2 2 paint=ink }}\n{}",
        scene(r#"use-scene "ball" 3 4"#)
    );
    let plain = format!(
        "{PALETTE}\nscene \"ball\" palette=\"p\" width=4 height=4 fps=4 duration=\"2s\" background=\"sky\" {{ rect x=0 y=0 width=2 height=2 paint=\"ink\" }}\n{}",
        scene(r#"use-scene "ball" x=3 y=4"#)
    );
    let options = LoadOptions {
        scene: Some("main"),
        ..LoadOptions::default()
    };
    assert_eq!(
        hashes(&load_scene(&sugared, options).unwrap().scene),
        hashes(&load_scene(&plain, options).unwrap().scene)
    );
}

#[test]
fn repeat_expands_in_place_with_k() {
    assert_same(
        "",
        r#"rect "=2 + $k*4" 1 2 2 paint=ink repeat=3
ellipse 1 9 4 4 paint=red"#,
        r#"rect x=2 y=1 width=2 height=2 paint="ink"
rect x=6 y=1 width=2 height=2 paint="ink"
rect x=10 y=1 width=2 height=2 paint="ink"
ellipse x=1 y=9 width=4 height=4 paint="red""#,
    );
}

#[test]
fn repeat_keeps_texture_seeds_of_the_handwritten_order() {
    assert_same(
        "",
        r#"rect "=$k*8" 0 8 8 paint=ink texture=0.8 repeat=3"#,
        r#"rect x=0 y=0 width=8 height=8 paint="ink" texture=0.8
rect x=8 y=0 width=8 height=8 paint="ink" texture=0.8
rect x=16 y=0 width=8 height=8 paint="ink" texture=0.8"#,
    );
}

#[test]
fn k_reaches_descendants_and_the_nearest_repeat_wins() {
    assert_same(
        "",
        r#"group "g$k" repeat=2 {
    rect "=$k*2" "=$k" 1 1 paint=ink
    rect "=$k*5" 9 1 1 paint=red repeat=2
}"#,
        r#"group "g0" {
    rect x=0 y=0 width=1 height=1 paint="ink"
    rect x=0 y=9 width=1 height=1 paint="red"
    rect x=5 y=9 width=1 height=1 paint="red"
}
group "g1" {
    rect x=2 y=1 width=1 height=1 paint="ink"
    rect x=0 y=9 width=1 height=1 paint="red"
    rect x=5 y=9 width=1 height=1 paint="red"
}"#,
    );
}

#[test]
fn ids_built_with_k_can_be_referenced() {
    assert_same(
        "",
        r#"rect "lamp$k" "=$k*3" 0 2 2 paint=ink repeat=3
rect "=@lamp2.x" 5 2 2 paint=red"#,
        r#"rect "lamp0" x=0 y=0 width=2 height=2 paint="ink"
rect "lamp1" x=3 y=0 width=2 height=2 paint="ink"
rect "lamp2" x=6 y=0 width=2 height=2 paint="ink"
rect x="=@lamp2.x" y=5 width=2 height=2 paint="red""#,
    );
}

const PIER: &str = r#"motif pier x tone="base" {
    rect $x 2 3 6 paint=ink
    rect "=$x-1" 8 5 2 paint=red opacity=$opacity
}
motif bay x opacity=1 {
    rect $x 0 2 2 paint=ink opacity=$opacity
    rect "=$x+3" 0 2 2 paint=red
}"#;

#[test]
fn a_motif_call_expands_to_its_body() {
    assert_same(
        PIER,
        "bay 4\nbay 12 opacity=0.5",
        r#"rect x=4 y=0 width=2 height=2 paint="ink" opacity=1
rect x=7 y=0 width=2 height=2 paint="red"
rect x=12 y=0 width=2 height=2 paint="ink" opacity=0.5
rect x=15 y=0 width=2 height=2 paint="red""#,
    );
}

#[test]
fn motifs_nest_and_take_repeat() {
    assert_same(
        r#"motif leg x { rect $x 0 1 4 paint=ink; }
motif table x { leg $x; leg "=$x+4"; }"#,
        r#"table "=$k*8" repeat=2"#,
        r#"rect x=0 y=0 width=1 height=4 paint="ink"
rect x=4 y=0 width=1 height=4 paint="ink"
rect x=8 y=0 width=1 height=4 paint="ink"
rect x=12 y=0 width=1 height=4 paint="ink""#,
    );
}

#[test]
fn a_motif_passes_negative_numbers_and_expressions_safely() {
    assert_same(
        r#"motif dot x { rect "=4 - $x" 0 1 1 paint=ink; }"#,
        "dot -2\ndot \"=1 + 1\"",
        r#"rect x=6 y=0 width=1 height=1 paint="ink"
rect x="=4 - (1 + 1)" y=0 width=1 height=1 paint="ink""#,
    );
}

#[test]
fn a_motif_substitutes_text_inside_strings() {
    assert_same(
        r#"motif tri w h paint { polygon 0 0 points="0,0 $w,0 0,$h" paint=$paint; }"#,
        "tri 6 4 paint=red.1",
        r#"polygon x=0 y=0 points="0,0 6,0 0,4" paint="red.1""#,
    );
}

#[test]
fn a_motif_keeps_scene_params_and_references_of_the_caller() {
    assert_same(
        r#"motif mark x { rect "=$x + @a.x" 3 1 1 paint=$tint; }"#,
        "param tint \"ink\"\nrect \"a\" 2 0 1 1 paint=red\nmark 4",
        r#"param tint "ink"
rect "a" x=2 y=0 width=1 height=1 paint="red"
rect x="=4 + @a.x" y=3 width=1 height=1 paint="$tint""#,
    );
}

#[test]
fn a_motif_can_hold_groups_placed_scenes_and_keyframes() {
    assert_same(
        r#"motif bob x { group x=$x y=0 width=4 height=4 clip=#true { rect 0 0 8 8 paint=ink { anim y { k "0s" $x; k "2s" 0; } } } }"#,
        "bob 3",
        r#"group x=3 y=0 width=4 height=4 clip=#true { rect x=0 y=0 width=8 height=8 paint="ink" { anim y { k "0s" 3; k "2s" 0; } } }"#,
    );
}

#[test]
fn a_motif_from_another_file_loads_and_errors_point_at_the_call() {
    let root = format!(
        "use \"parts.kdl\"\n{PALETTE}\n{}",
        scene("rect 0 0 1 1 paint=ink\ndot 5")
    );
    let parts = "motif dot x { rect $x 0 1 1 paint=ink; }";
    let mut sources = MemorySources::default()
        .with("root.kdl", &root)
        .with("parts.kdl", parts);
    let loaded = load(Path::new("root.kdl"), &mut sources, LoadOptions::default()).unwrap();
    assert_eq!(loaded.motifs.len(), 1);
    let plain = file(
        "",
        "rect x=0 y=0 width=1 height=1 paint=\"ink\"\nrect x=5 y=0 width=1 height=1 paint=\"ink\"",
    );
    assert_eq!(hashes(&loaded.scene), render(&plain));
    let call_offset = root.find("dot 5").unwrap();
    assert!(
        loaded
            .origins
            .iter()
            .any(|origin| origin.file == 0 && origin.offset == call_offset)
    );

    let bad = "motif dot x { rect $x 0 1 1 paint=nope; }";
    let mut sources = MemorySources::default()
        .with("root.kdl", &root)
        .with("parts.kdl", bad);
    let failed = load(Path::new("root.kdl"), &mut sources, LoadOptions::default()).unwrap_err();
    let diagnostic = &failed.diagnostics[0];
    assert_eq!(diagnostic.file, 0);
    assert_eq!(diagnostic.offset, call_offset);
}

#[test]
fn folding_only_touches_plain_arithmetic() {
    assert_same(
        r#"motif a x s { rect "=$x + t" 0 1 1 paint=ink gradient="down" gradient-steps="=$s-1"; }"#,
        "a 1 3",
        r#"rect x="=1 + t" y=0 width=1 height=1 paint="ink" gradient="down" gradient-steps=2"#,
    );
    assert_same(
        r#"motif b x { rect "=1/$x" 0 1 1 paint=ink; rect "=$x*-10" 2 1 1 paint=red; }"#,
        "b 0",
        r#"rect x="=1/0" y=0 width=1 height=1 paint="ink"
rect x=0 y=2 width=1 height=1 paint="red""#,
    );
    assert_same(
        r#"motif c n { particles 0 0 8 8 paint=ink count="=$n*2"; }"#,
        "c 4",
        r#"particles x=0 y=0 width=8 height=8 paint="ink" count=8"#,
    );
}

#[test]
fn a_range_argument_stays_a_coordinate() {
    let motif = r#"motif puff x y { blob $x $y circles="0,0,3 2,0,2" paint=ink; }"#;
    assert_same(
        motif,
        r#"puff "5..25" 4"#,
        r#"blob x="5..25" y=4 circles="0,0,3 2,0,2" paint="ink""#,
    );
    let body = r#"ellipse "5..25" 44 "8..14" 6 paint=ink"#;
    assert_error("", body, "is missing `height`");
}

#[test]
fn a_one_line_motif_parses_with_spans_on_the_call() {
    let text = file("motif a x { rect $x 0 1 1 paint=ink }", "a 3");
    let loaded = load_scene(&text, LoadOptions::default()).unwrap();
    let call = text.rfind("a 3").unwrap();
    assert_eq!(loaded.origins.last().unwrap().offset, call);
    assert_eq!(loaded.motifs[0].name, "a");
    assert_eq!(loaded.motifs[0].required, ["x"]);
}

#[test]
fn a_file_without_shorthand_is_unchanged() {
    let text = file("", r#"rect "a" x=1 y=2 width=3 height=4 paint="ink""#);
    let loaded = load_scene(&text, LoadOptions::default()).unwrap();
    assert_eq!(loaded.motifs.len(), 0);
    assert_eq!(
        loaded.origins.last().unwrap().offset,
        text.rfind("rect").unwrap()
    );
}
