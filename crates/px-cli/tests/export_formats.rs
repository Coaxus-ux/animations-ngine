#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use px_editor::domain::Document;
use px_editor::domain::export::{
    ExportJob, ExportKind, FrameScope, KIND_NAMES, LAYOUT_NAMES, run, sequence_name,
};
use px_editor::domain::frames::render_frames;
use px_editor::ports::MemoryFiles;
use px_format::{LoadOptions, Work};

const EXAMPLE: &str = "hello";
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];

fn example() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../examples/{EXAMPLE}.kdl"))
}

fn fresh_dir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pxe-{test}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn pxe(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pxe"))
        .arg("export")
        .arg(example())
        .args(["--scale", "1"])
        .args(args)
        .output()
        .unwrap()
}

fn export(args: &[&str]) {
    let output = pxe(args);
    assert!(output.status.success(), "{output:?}");
}

fn frame_count() -> usize {
    match px_format::load_work_file(&example(), LoadOptions::default()).unwrap() {
        Work::Scene(loaded) => loaded.scene.timing().duration.get() as usize,
        Work::Sequence(_) => unreachable!("{EXAMPLE} is a scene"),
    }
}

fn document() -> Document {
    let path = example().canonicalize().unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    let mut files = MemoryFiles::default().with(path.clone(), text);
    Document::open(&path, &mut files).unwrap()
}

#[test]
fn a_sheet_export_writes_png_and_json() {
    let dir = fresh_dir("sheet");
    let sheet = dir.join("sheet.png");
    let out = sheet.to_str().unwrap();
    export(&[
        "--format",
        "sheet",
        "--layout",
        "rows",
        "--count",
        "6",
        "--padding",
        "1",
        "--border",
        "2",
        "--out",
        out,
    ]);
    let png = std::fs::read(&sheet).unwrap();
    let json = std::fs::read_to_string(dir.join("sheet.json")).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(png[..8], PNG_SIGNATURE);
    assert!(json.contains("\"frames\""), "{json}");
    assert!(json.contains("\"image\":\"sheet.png\""), "{json}");
}

#[test]
fn a_png_sequence_names_frames() {
    let dir = fresh_dir("sequence");
    let out = dir.join("walk.png");
    export(&["--format", "sequence", "--out", out.to_str().unwrap()]);
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    std::fs::remove_dir_all(&dir).unwrap();
    names.sort();
    let expected: Vec<String> = (0..frame_count())
        .map(|index| sequence_name("walk", index))
        .collect();
    assert_eq!(names, expected);
}

#[test]
fn a_transparent_png_has_alpha() {
    let dir = fresh_dir("transparent");
    let out = dir.join("frame.png");
    export(&[
        "--format",
        "png",
        "--frame",
        "3",
        "--transparent",
        "--out",
        out.to_str().unwrap(),
    ]);
    let written = std::fs::read(&out).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    let document = document();
    let hidden = BTreeSet::new();
    let frames = render_frames(&document, &hidden, true, FrameScope::Current(3)).unwrap();
    assert!(
        frames[0].pixels.iter().any(Option::is_none),
        "the keyed render leaves no transparent pixel"
    );
    let job = ExportJob {
        kind: ExportKind::Png,
        scale: 1,
        detail: px_engine::Detail::NATIVE,
        transparent: true,
        frames: FrameScope::Current(3),
        out: out.clone(),
        trim: false,
        split_layers: false,
    };
    let expected = run(&job, &document, &hidden).unwrap();
    assert_eq!(written, expected[0].bytes);
}

#[test]
fn the_default_gif_bytes_are_unchanged() {
    let dir = fresh_dir("default-gif");
    let plain = dir.join("plain.gif");
    let explicit = dir.join("explicit.gif");
    export(&["--out", plain.to_str().unwrap()]);
    export(&[
        "--format",
        "gif",
        "--jobs",
        "2",
        "--out",
        explicit.to_str().unwrap(),
    ]);
    let plain = std::fs::read(&plain).unwrap();
    let explicit = std::fs::read(&explicit).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(&plain[..6], b"GIF89a");
    assert_eq!(plain, explicit);
    let mut decoder = gif::DecodeOptions::new().read_info(&plain[..]).unwrap();
    let mut frames = 0;
    while decoder.read_next_frame().unwrap().is_some() {
        frames += 1;
    }
    assert_eq!(frames, frame_count());
}

#[test]
fn params_are_refused_outside_gif() {
    let output = pxe(&["--format", "sheet", "--param", "x=1"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("params are not supported for this format yet"),
        "{stderr}"
    );
}

#[test]
fn sheet_flags_are_refused_for_other_formats() {
    refused(
        &["--format", "png", "--layout", "rows"],
        "--layout applies to --format sheet",
    );
}

#[test]
fn flags_a_format_ignores_are_refused() {
    refused(
        &["--format", "gif", "--trim"],
        "--trim applies to --format png, sequence or sheet",
    );
    refused(
        &["--format", "gif", "--split-layers"],
        "--split-layers applies to --format sequence or sheet",
    );
    refused(
        &["--format", "png", "--split-layers"],
        "--split-layers applies to --format sequence or sheet",
    );
    refused(
        &["--format", "png", "--jobs", "2"],
        "--jobs applies to --format gif without --transparent or --frame",
    );
    refused(
        &["--format", "gif", "--transparent", "--jobs", "2"],
        "--jobs applies to --format gif without --transparent or --frame",
    );
}

#[test]
fn format_and_layout_names_are_the_editors() {
    refused(
        &["--format", "bogus"],
        &format!("expected one of {}", KIND_NAMES.join(", ")),
    );
    refused(
        &["--format", "sheet", "--layout", "bogus"],
        &format!("expected one of {}", LAYOUT_NAMES.join(", ")),
    );
    refused(
        &["--format", "sheet", "--layout", "rows"],
        "--layout rows needs --count (columns)",
    );
}

#[test]
fn an_unknown_scene_is_reported_like_the_plain_gif_path() {
    let plain = pxe(&["--scene", "nope"]);
    let editor = pxe(&["--format", "gif", "--transparent", "--scene", "nope"]);
    assert!(!plain.status.success());
    assert!(!editor.status.success());
    let plain = String::from_utf8(plain.stderr).unwrap();
    let editor = String::from_utf8(editor.stderr).unwrap();
    assert!(
        plain.contains("no scene or sequence named \"nope\""),
        "{plain}"
    );
    assert_eq!(plain, editor);
}

#[test]
fn a_non_first_scene_names_the_flags_that_forced_the_editor_path() {
    let output = Command::new(env!("CARGO_BIN_EXE_pxe"))
        .arg("export")
        .arg(example().with_file_name("town-festival.kdl"))
        .args(["--scale", "1", "--transparent", "--scene", "festival-sky"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains(
            "--scene festival-sky: only the file's first scene (\"town-festival\") can be \
             exported with --transparent yet"
        ),
        "{stderr}"
    );
}

fn refused(args: &[&str], message: &str) {
    let output = pxe(args);
    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains(message), "{stderr}");
}
