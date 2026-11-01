#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use std::path::PathBuf;
use std::process::{Command, Output};

const DOT: &str = r##"palette "p" { ramp "sky" "#000000"; ramp "ink" "#ffffff" }
scene "dot" palette="p" width=8 height=4 fps=4 duration="1s" background="sky" {
    rect x="=t * 4" y=0 width=1 height=1 paint="ink"
}
"##;

const FILM: &str = r#"use "dot.kdl"
sequence "film" frame-budget="10s" {
    shot "dot" duration="1s"
}
"#;

fn film(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pxe-bench-{test}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("dot.kdl"), DOT).unwrap();
    let path = dir.join("film.kdl");
    std::fs::write(&path, FILM).unwrap();
    path
}

fn bench(test: &str, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pxe"))
        .arg("bench")
        .arg(film(test))
        .args(["--rounds", "1"])
        .args(extra)
        .output()
        .unwrap()
}

#[test]
fn a_sequence_is_timed_against_its_own_frame_budget() {
    let own = bench("own", &[]);
    assert!(own.status.success(), "{own:?}");
    assert!(
        String::from_utf8_lossy(&own.stdout).contains("none over budget"),
        "{own:?}"
    );
}

#[test]
fn an_explicit_budget_wins_over_the_sequence_s() {
    let strict = bench("strict", &["--budget-ms", "0"]);
    assert_eq!(strict.status.code(), Some(4), "{strict:?}");
    assert!(
        String::from_utf8_lossy(&strict.stderr).contains("over the frame budget:"),
        "{strict:?}"
    );
}
