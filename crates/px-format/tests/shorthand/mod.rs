#![allow(
    dead_code,
    clippy::unwrap_used,
    reason = "test helpers; not every test file uses every helper"
)]

use px_core::{Frame, hash_cells};
use px_engine::{FrameBuffer, Scene};
use px_format::{LoadOptions, Severity, load_scene};

pub const PALETTE: &str = r##"palette "p" { ramp "sky" "#000000" "#111111"; ramp "ink" "#ffffff" "#eeeeee"; ramp "red" "#ff0000" "#aa0000" }"##;

pub fn scene(body: &str) -> String {
    format!(
        "scene \"main\" palette=\"p\" width=32 height=16 fps=4 duration=\"2s\" loop=#true background=\"sky\" {{\n{body}\n}}\n"
    )
}

pub fn file(motifs: &str, body: &str) -> String {
    format!("{PALETTE}\n{motifs}\n{}", scene(body))
}

pub fn hashes(scene: &Scene) -> Vec<u64> {
    let mut buffer = FrameBuffer::new(scene);
    (0..8)
        .map(|index| hash_cells(buffer.render(scene, Frame::new(index)).unwrap().cells()))
        .collect()
}

pub fn render(text: &str) -> Vec<u64> {
    hashes(&load_scene(text, LoadOptions::default()).unwrap().scene)
}

pub fn assert_same(motifs: &str, sugared: &str, handwritten: &str) {
    assert_eq!(
        render(&file(motifs, sugared)),
        render(&file("", handwritten)),
        "{sugared}"
    );
}

pub fn errors(motifs: &str, body: &str) -> Vec<String> {
    load_scene(&file(motifs, body), LoadOptions::default())
        .unwrap_err()
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| diagnostic.message.clone())
        .collect()
}

pub fn assert_error(motifs: &str, body: &str, expected: &str) {
    let found = errors(motifs, body);
    assert!(
        found.iter().any(|message| message.contains(expected)),
        "wanted `{expected}` in {found:?}"
    );
}
