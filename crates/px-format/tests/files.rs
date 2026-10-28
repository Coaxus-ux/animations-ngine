#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use std::path::Path;

use px_format::{LoadOptions, MemorySources, load};

const PALETTE: &str = r##"palette "p" { ramp "sky" "#000000" "#111111"; ramp "ink" "#ffffff" }"##;

fn scene(name: &str, body: &str) -> String {
    format!(
        "scene \"{name}\" palette=\"p\" width=8 height=8 duration=\"2f\" background=\"sky\" {{\n{body}\n}}\n"
    )
}

fn errors(sources: MemorySources, root: &str) -> Vec<String> {
    let mut sources = sources;
    let error = load(Path::new(root), &mut sources, LoadOptions::default()).unwrap_err();
    error
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.display(&error.files))
        .collect()
}

#[test]
fn palettes_and_scenes_come_from_used_files() {
    let mut sources = MemorySources::default()
        .with(
            "town/main.kdl",
            format!("use \"parts/palette.kdl\"\n{}", scene("main", "")),
        )
        .with("town/parts/palette.kdl", PALETTE);
    let loaded = load(
        Path::new("town/main.kdl"),
        &mut sources,
        LoadOptions::default(),
    )
    .unwrap();
    assert_eq!(loaded.scene.name(), "main");
    let paths: Vec<_> = loaded.files.iter().map(|file| file.path.clone()).collect();
    assert_eq!(
        paths,
        [
            Path::new("town/main.kdl"),
            Path::new("town/parts/palette.kdl")
        ]
    );
}

#[test]
fn a_used_file_can_be_reached_twice_without_a_cycle() {
    let mut sources = MemorySources::default()
        .with(
            "main.kdl",
            format!("use \"a.kdl\"\nuse \"b.kdl\"\n{}", scene("main", "")),
        )
        .with("a.kdl", "use \"p.kdl\"")
        .with("b.kdl", "use \"p.kdl\"")
        .with("p.kdl", PALETTE);
    let loaded = load(Path::new("main.kdl"), &mut sources, LoadOptions::default()).unwrap();
    assert_eq!(loaded.files.len(), 4);
}

#[test]
fn use_cycles_are_errors_that_show_the_chain() {
    let sources = MemorySources::default()
        .with(
            "main.kdl",
            format!("use \"a.kdl\"\n{PALETTE}\n{}", scene("main", "")),
        )
        .with("a.kdl", "use \"main.kdl\"");
    assert_eq!(
        errors(sources, "main.kdl"),
        ["a.kdl:1:1 error `use` cycle: main.kdl → a.kdl → main.kdl"]
    );
}

#[test]
fn diagnostics_name_the_file_they_come_from() {
    let sources = MemorySources::default()
        .with(
            "main.kdl",
            format!("use \"missing.kdl\"\nuse \"p.kdl\"\n{}", scene("main", "")),
        )
        .with("p.kdl", "palette \"p\" { ramp \"sky\" \"#zzzzzz\" }");
    let errors = errors(sources, "main.kdl");
    assert!(
        errors
            .iter()
            .any(|e| e.starts_with("main.kdl:1:1 error cannot read missing.kdl")),
        "{errors:?}"
    );
    assert!(
        errors.iter().any(|e| e.starts_with("p.kdl:1:")),
        "{errors:?}"
    );
}

#[test]
fn scenes_from_used_files_can_be_chosen_by_name() {
    let mut sources = MemorySources::default()
        .with(
            "main.kdl",
            format!("use \"parts.kdl\"\n{PALETTE}\n{}", scene("main", "")),
        )
        .with("parts.kdl", scene("walker", ""));
    let options = LoadOptions {
        scene: Some("walker"),
        ..LoadOptions::default()
    };
    let loaded = load(Path::new("main.kdl"), &mut sources, options).unwrap();
    assert_eq!(loaded.scene.name(), "walker");
}
