#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SIDE: u32 = 64;

fn example() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello.kdl")
}

fn fresh_dir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pxe-detail-{test}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn pxe(command: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pxe"))
        .arg(command)
        .arg(example())
        .args(["--scale", "1"])
        .args(args)
        .output()
        .unwrap()
}

fn written(test: &str, file: &str, command: &str, args: &[&str]) -> Vec<u8> {
    let dir = fresh_dir(test);
    let out = dir.join(file);
    let mut all = args.to_vec();
    all.extend(["--out", out.to_str().unwrap()]);
    let output = pxe(command, &all);
    assert!(output.status.success(), "{output:?}");
    let bytes = std::fs::read(&out).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    bytes
}

fn png_size(bytes: &[u8]) -> (u32, u32) {
    let word = |at: usize| u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
    (word(16), word(20))
}

fn gif_size(bytes: &[u8]) -> (u32, u32) {
    let half = |at: usize| u32::from(u16::from_le_bytes([bytes[at], bytes[at + 1]]));
    (half(6), half(8))
}

#[test]
fn render_redraws_the_scene_at_the_requested_detail() {
    let native = written("native", "frame.png", "render", &[]);
    let doubled = written("render", "frame.png", "render", &["--detail", "2"]);
    assert_eq!(png_size(&native), (SIDE, SIDE));
    assert_eq!(png_size(&doubled), (2 * SIDE, 2 * SIDE));
}

#[test]
fn every_export_format_takes_the_detail() {
    let gif = written("gif", "hello.gif", "export", &["--detail", "2"]);
    assert_eq!(gif_size(&gif), (2 * SIDE, 2 * SIDE));
    let png = written(
        "png",
        "hello.png",
        "export",
        &["--format", "png", "--detail", "1.5"],
    );
    assert_eq!(png_size(&png), (SIDE * 3 / 2, SIDE * 3 / 2));
}

#[test]
fn a_detail_outside_one_to_eight_is_refused() {
    for detail in ["0.5", "9", "nan", "big"] {
        let output = pxe("render", &["--detail", detail]);
        assert!(!output.status.success(), "--detail {detail} was accepted");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("detail"), "{error}");
    }
}
