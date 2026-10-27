//! Loading scene files: the example, and every kind of diagnostic.
#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use px_format::{Severity, load_scene};

const HELLO: &str = include_str!("../../../examples/hello.kdl");

const PALETTE: &str = r##"palette "p" { ramp "sky" "#000000" "#111111" "#222222"; ramp "ink" "#ffffff" }
"##;

fn scene(body: &str, props: &str) -> String {
    format!(
        "{PALETTE}scene \"s\" palette=\"p\" width=8 height=8 duration=\"2f\" background=\"sky\" {props} {{\n{body}\n}}\n"
    )
}

/// Loads and returns every diagnostic rendered as `line:col severity message`.
fn diagnostics(source: &str) -> Vec<String> {
    let diagnostics = match load_scene(source, None) {
        Ok(loaded) => loaded.warnings,
        Err(error) => error.diagnostics,
    };
    diagnostics
        .iter()
        .map(|d| d.render("f", source).trim_start_matches("f:").to_owned())
        .collect()
}

#[test]
fn example_scene_loads_without_warnings() {
    let loaded = load_scene(HELLO, None).unwrap();
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    let scene = loaded.scene;
    assert_eq!(scene.name(), "hello");
    assert_eq!((scene.size().width(), scene.size().height()), (64, 64));
    assert_eq!(scene.timing().duration.get(), 24);
    assert_eq!(scene.shapes().len(), 7);
}

#[test]
fn minimal_scene_loads_with_defaults() {
    let loaded = load_scene(
        &scene("rect x=0 y=0 width=1 height=1 paint=\"ink\"", ""),
        None,
    )
    .unwrap();
    assert_eq!(loaded.scene.timing().fps.get(), 12);
    assert_eq!(loaded.scene.background().tone.index(), 1);
}

#[test]
fn unknown_property_is_a_warning_not_an_error() {
    let found = diagnostics(&scene(
        "rect x=0 y=0 width=1 height=1 paint=\"ink\" glow=1",
        "",
    ));
    assert_eq!(found.len(), 1);
    assert!(
        found[0].starts_with("3:") && found[0].contains("warning unknown property `glow`"),
        "{found:?}"
    );
}

#[test]
fn errors_point_at_the_offending_line() {
    let found = diagnostics(&scene("rect x=0 y=0 width=1 paint=\"ink\"\nblob x=1", ""));
    assert!(
        found
            .iter()
            .any(|d| d.starts_with("3:1 error") && d.contains("missing `height`")),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|d| d.starts_with("4:1 error") && d.contains("unknown shape `blob`")),
        "{found:?}"
    );
}

#[test]
fn bad_expressions_paints_and_references_are_errors() {
    let body = "rect x=\"=t +\" y=0 width=1 height=1 paint=\"ink\"\nrect x=0 y=0 width=1 height=1 paint=\"sky.hi\"\nrect x=0 y=0 width=1 height=1 paint=\"sky.9\"\nrect x=0 y=0 width=1 height=1 paint=\"sea\"";
    let found = diagnostics(&scene(body, ""));
    let has = |needle: &str| found.iter().any(|d| d.contains(needle));
    assert!(has("invalid expression `t +`"), "{found:?}");
    assert!(has("invalid tone \"9\""), "{found:?}");
    assert!(has("unknown ramp \"sea\""), "{found:?}");
    assert!(
        !has("sky.hi"),
        "named tones resolve on 3-tone ramps: {found:?}"
    );
}

#[test]
fn unknown_palette_and_top_level_nodes_are_errors() {
    let source =
        "scene \"s\" palette=\"nope\" width=8 height=8 duration=\"1f\" background=\"x\"\nlight 1\n";
    let found = diagnostics(source);
    assert!(
        found.iter().any(|d| d.contains("unknown palette \"nope\"")),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|d| d.starts_with("2:1 error unknown top-level node `light`")),
        "{found:?}"
    );
}

#[test]
fn durations_need_units_and_round_with_a_warning() {
    let unitless = scene("", "").replace("duration=\"2f\"", "duration=\"2\"");
    assert!(
        diagnostics(&unitless)
            .iter()
            .any(|d| d.contains("invalid duration \"2\""))
    );
    let fractional = scene("", "").replace("duration=\"2f\"", "duration=\"0.21s\"");
    let loaded = load_scene(&fractional, None).unwrap();
    assert_eq!(loaded.scene.timing().duration.get(), 3);
    assert_eq!(
        loaded.warnings.first().map(|w| w.severity),
        Some(Severity::Warning)
    );
}

#[test]
fn syntax_errors_are_reported() {
    let found = diagnostics("scene \"s\" {\n");
    assert!(
        !found.is_empty() && found.iter().all(|d| d.contains("error")),
        "{found:?}"
    );
}

#[test]
fn several_scenes_need_a_name() {
    let two = format!(
        "{PALETTE}scene \"a\" palette=\"p\" width=1 height=1 duration=\"1f\" background=\"sky\"\nscene \"b\" palette=\"p\" width=2 height=2 duration=\"1f\" background=\"sky\"\n"
    );
    assert!(load_scene(&two, None).is_err());
    let loaded = load_scene(&two, Some("b")).unwrap();
    assert_eq!(loaded.scene.size().width(), 2);
    assert!(load_scene(&two, Some("c")).is_err());
}
