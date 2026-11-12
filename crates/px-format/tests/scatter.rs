#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use std::collections::BTreeSet;

use px_core::{Frame, Paint};
use px_engine::FrameBuffer;
use px_format::{LoadOptions, Severity, load_scene};

const PALETTE: &str = r##"palette "p" { ramp "sky" "#000000" "#111111"; ramp "ink" "#ffffff" "#eeeeee"; ramp "red" "#ff0000" "#aa0000" }"##;

fn file(body: &str) -> String {
    format!(
        "{PALETTE}\nscene \"main\" palette=\"p\" width=32 height=16 fps=4 duration=\"1s\" background=\"sky\" {{\n{body}\n}}\n"
    )
}

fn cells(body: &str) -> Vec<Paint> {
    let scene = load_scene(&file(body), LoadOptions::default())
        .unwrap()
        .scene;
    let mut buffer = FrameBuffer::new(&scene);
    buffer
        .render(&scene, Frame::new(0))
        .unwrap()
        .cells()
        .to_vec()
}

fn lit(body: &str, row: usize) -> BTreeSet<usize> {
    let cells = cells(body);
    let background = cells[31 * 16 - 1];
    (0..32)
        .filter(|column| cells[row * 32 + column] != background)
        .collect()
}

fn errors(body: &str) -> Vec<String> {
    load_scene(&file(body), LoadOptions::default())
        .unwrap_err()
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| diagnostic.message.clone())
        .collect()
}

fn assert_error(body: &str, expected: &str) {
    let found = errors(body);
    assert!(
        found.iter().any(|message| message.contains(expected)),
        "wanted `{expected}` in {found:?}"
    );
}

const DOTS: &str = r#"scatter 12 seed=4 x="2..20" { rect $x 0 1 1 paint=ink }"#;

#[test]
fn whole_ranges_stay_inside_their_ends_and_vary() {
    let columns = lit(DOTS, 0);
    assert!(columns.len() > 4, "{columns:?}");
    assert!(
        columns.iter().all(|column| (2..=20).contains(column)),
        "{columns:?}"
    );
}

#[test]
fn the_same_text_always_scatters_the_same_way() {
    assert_eq!(cells(DOTS), cells(DOTS));
}

#[test]
fn the_seed_reshuffles_the_values() {
    let other = DOTS.replace("seed=4", "seed=5");
    assert_ne!(lit(DOTS, 0), lit(&other, 0));
}

#[test]
fn shapes_written_elsewhere_do_not_move_scattered_ones() {
    let before = format!("rect 0 9 4 4 paint=red\n{DOTS}");
    assert_eq!(lit(DOTS, 0), lit(&before, 0));
}

#[test]
fn fixed_spreads_and_k_match_handwritten_shapes() {
    assert_eq!(
        cells(r#"scatter 3 y="2..2" tone="ink|ink" { rect "=$k*4" $y 2 2 paint=$tone }"#),
        cells("rect 0 2 2 2 paint=ink\nrect 4 2 2 2 paint=ink\nrect 8 2 2 2 paint=ink")
    );
}

#[test]
fn choices_pick_numbers_and_names() {
    let body =
        r#"scatter 24 seed=2 x="0..31" tone="ink|red" h="1|2" { rect $x 0 1 $h paint=$tone }"#;
    let cells = cells(body);
    let background = cells[31 * 16 - 1];
    let paints: BTreeSet<_> = cells
        .iter()
        .filter(|paint| **paint != background)
        .map(|paint| paint.ramp)
        .collect();
    assert_eq!(paints.len(), 2, "both choices should appear");
    assert!(!lit(body, 1).is_empty(), "some copies should be two tall");
}

#[test]
fn fractional_ranges_and_motif_calls_work() {
    let text = format!(
        "{PALETTE}\nmotif dot x {{ rect $x 3 1 1 paint=red }}\nscene \"main\" palette=\"p\" width=32 height=16 fps=4 duration=\"1s\" background=\"sky\" {{\nscatter 8 x=\"0.5..9.5\" {{ dot \"=round($x)\" }}\n}}\n"
    );
    let loaded = load_scene(&text, LoadOptions::default()).unwrap();
    assert_eq!(loaded.scene.shapes().len(), 8);
}

#[test]
fn every_misuse_reports_an_error() {
    assert_error(
        r#"scatter x="0..3" { rect $x 0 1 1 paint=ink }"#,
        "needs a count",
    );
    assert_error(
        "scatter 0 { rect 0 0 1 1 paint=ink }",
        "whole number in 1..=4096",
    );
    assert_error(
        "scatter 5000 { rect 0 0 1 1 paint=ink }",
        "whole number in 1..=4096",
    );
    assert_error("scatter 2 3 { rect 0 0 1 1 paint=ink }", "one count");
    assert_error(
        "scatter 2 seed=-1 { rect 0 0 1 1 paint=ink }",
        "whole number in 0..",
    );
    assert_error("scatter 2", "has no body");
    assert_error(
        r#"scatter 2 x="a..b" { rect 0 0 1 1 paint=ink }"#,
        "not a range",
    );
    assert_error(
        r#"scatter 2 x="only" { rect 0 0 1 1 paint=ink }"#,
        "at least two choices",
    );
    assert_error(
        r#"scatter 2 x="a||b" { rect 0 0 1 1 paint=ink }"#,
        "at least two choices",
    );
    assert_error("scatter 2 x=3 { rect 0 0 1 1 paint=ink }", "needs a range");
    assert_error(
        r#"scatter 2 k="0..3" { rect 0 0 1 1 paint=ink }"#,
        "cannot be a scatter value",
    );
    assert_error(
        r#"scatter 2 X="0..3" { rect 0 0 1 1 paint=ink }"#,
        "cannot be a scatter value",
    );
    assert_error("scatter 2 { layer \"l\" { } }", "cannot hold `layer`");
    assert_error(
        "scatter 2 { scatter 2 { rect 0 0 1 1 paint=ink } }",
        "cannot hold `scatter`",
    );
    assert_error(
        "scatter 2 repeat=2 { rect 0 0 1 1 paint=ink }",
        "`repeat` cannot be a scatter value",
    );
}
