#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use common::open;
use px_editor::app::Intent;
use px_editor::domain::Document;
use px_editor::domain::export::{ExportJob, ExportKind, FrameScope, frame_tags};
use px_editor::domain::frames::render_frames;
use px_editor::ports::MemoryFiles;
use px_export::{Padding, SheetLayout, trim_bounds};
use px_format::Marker;

const COLUMNS: u16 = 2;

fn sheet_job(out: &str) -> ExportJob {
    ExportJob {
        kind: ExportKind::Sheet {
            layout: SheetLayout::Rows { columns: COLUMNS },
            padding: Padding::default(),
            json: true,
        },
        out: PathBuf::from(out),
        ..ExportJob::default()
    }
}

fn text(files: &common::MemoryFiles, path: &str) -> String {
    String::from_utf8(files.bytes[Path::new(path)].clone()).unwrap()
}

fn png_size(bytes: &[u8]) -> (u32, u32) {
    let field = |at: usize| u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
    (field(16), field(20))
}

fn graphic_controls(gif: &[u8]) -> Vec<u8> {
    gif.windows(4)
        .filter(|block| block[..3] == [0x21, 0xF9, 0x04])
        .map(|block| block[3])
        .collect()
}

#[test]
fn a_transparent_gif_export_marks_frames_transparent_with_background_disposal() {
    let (mut app, mut files, main) = open();
    let gif = |transparent, out: &str| ExportJob {
        kind: ExportKind::Gif,
        transparent,
        out: PathBuf::from(out),
        ..ExportJob::default()
    };
    app.dispatch(
        Intent::Export(gif(false, "out/opaque.gif")),
        main,
        &mut files,
    );
    app.dispatch(Intent::Export(gif(true, "out/keyed.gif")), main, &mut files);
    let opaque = graphic_controls(&files.bytes[Path::new("out/opaque.gif")]);
    let keyed = graphic_controls(&files.bytes[Path::new("out/keyed.gif")]);
    assert_eq!(keyed.len(), 18);
    assert_eq!(keyed.len(), opaque.len());
    assert!(
        keyed
            .iter()
            .all(|packed| (packed >> 2) & 7 == 2 && packed & 1 == 1)
    );
    assert_eq!(
        opaque[0] & 1,
        0,
        "the first opaque frame has no transparent index"
    );
    assert!(opaque.iter().all(|packed| (packed >> 2) & 7 != 2));
}

#[test]
fn trim_crops_every_frame_to_the_common_bounds_and_records_the_offset() {
    let (mut app, mut files, main) = open();
    app.dispatch(Intent::ToggleHidden("room".to_owned()), main, &mut files);
    let hidden: BTreeSet<String> = ["room".to_owned()].into_iter().collect();
    let frames = render_frames(app.document().unwrap(), &hidden, true, FrameScope::All).unwrap();
    let bounds = trim_bounds(&frames).unwrap();
    assert!(bounds.w < 64 && bounds.h < 48, "{bounds:?}");
    app.dispatch(
        Intent::Export(ExportJob {
            trim: true,
            ..sheet_job("out/room.png")
        }),
        main,
        &mut files,
    );
    assert_eq!(app.editing().status.text, "Exported 2 files to out");
    let size = png_size(&files.bytes[Path::new("out/room.png")]);
    assert_eq!(size, (u32::from(COLUMNS) * bounds.w, 9 * bounds.h));
    let json = text(&files, "out/room.json");
    assert!(json.contains(r#""trimmed":true"#), "{json}");
    assert!(
        json.contains(&format!(
            r#""spriteSourceSize":{{"x":{},"y":{},"w":{},"h":{}}},"sourceSize":{{"w":64,"h":48}}"#,
            bounds.x, bounds.y, bounds.w, bounds.h
        )),
        "{json}"
    );
}

#[test]
fn split_layers_writes_one_sheet_per_visible_layer() {
    let (mut app, mut files, main) = open();
    app.dispatch(Intent::ToggleHidden("plant".to_owned()), main, &mut files);
    app.dispatch(
        Intent::Export(ExportJob {
            split_layers: true,
            ..sheet_job("out/room.png")
        }),
        main,
        &mut files,
    );
    assert_eq!(app.editing().status.text, "Exported 6 files to out");
    let names: Vec<&Path> = files.bytes.keys().map(PathBuf::as_path).collect();
    let expected = [
        "out/room-cat.json",
        "out/room-cat.png",
        "out/room-layer-1.json",
        "out/room-layer-1.png",
        "out/room-room.json",
        "out/room-room.png",
    ];
    assert_eq!(names, expected.map(Path::new));
    let cat = text(&files, "out/room-cat.json");
    assert!(cat.contains(r#""layers":[{"name":"cat","#), "{cat}");
    assert!(!cat.contains(r#""name":"room""#), "{cat}");
    assert_ne!(
        files.bytes[Path::new("out/room-cat.png")],
        files.bytes[Path::new("out/room-room.png")]
    );

    files.bytes.clear();
    app.dispatch(
        Intent::Export(ExportJob {
            kind: ExportKind::PngSequence,
            split_layers: true,
            frames: FrameScope::Current(0),
            ..sheet_job("out/room.png")
        }),
        main,
        &mut files,
    );
    let names: Vec<&Path> = files.bytes.keys().map(PathBuf::as_path).collect();
    assert_eq!(
        names,
        [
            "out/room-cat-0000.png",
            "out/room-layer-1-0000.png",
            "out/room-room-0000.png"
        ]
        .map(Path::new)
    );
}

#[test]
fn markers_become_frame_tags_in_order() {
    let marker = |name: &str, at: f64| Marker {
        name: name.to_owned(),
        at,
    };
    let markers = [
        marker("b", 1.0),
        marker("a", 0.5),
        marker("c", 1.0),
        marker("d", 9.0),
    ];
    let tags = frame_tags(&markers, 12, 24);
    let expected = [("a", 6, 11), ("c", 12, 22), ("d", 23, 23)];
    let got: Vec<(&str, u32, u32)> = tags
        .iter()
        .map(|(name, from, to)| (name.as_str(), *from, *to))
        .collect();
    assert_eq!(got, expected);
    assert_eq!(frame_tags(&markers, 12, 0), []);

    let text = concat!(
        "palette \"p\" {\n    ramp \"ink\" \"#000000\" \"#ffffff\"\n}\n",
        "scene \"tagged\" palette=\"p\" width=4 height=4 fps=12 duration=\"24f\" background=\"ink.0\" {\n",
        "    timeline {\n        marker \"run\" at=\"1s\"\n        marker \"idle\" at=\"0s\"\n    }\n",
        "    rect x=1 y=1 width=2 height=2 paint=\"ink.1\"\n}\n"
    );
    let mut files = common::MemoryFiles::default();
    let document =
        Document::from_text(Path::new("tagged.kdl"), text.to_owned(), &mut files).unwrap();
    assert_eq!(document.errors(), [] as [String; 0]);
    let outputs =
        px_editor::domain::export::run(&sheet_job("out/tagged.png"), &document, &BTreeSet::new())
            .unwrap();
    let json = String::from_utf8(outputs[1].bytes.clone()).unwrap();
    assert!(
        json.contains(concat!(
            r##""frameTags":[{"name":"idle","from":0,"to":11,"direction":"forward","color":"#000000ff","repeat":"0"},"##,
            r##"{"name":"run","from":12,"to":23,"direction":"forward","color":"#000000ff","repeat":"0"}]"##
        )),
        "{json}"
    );
}

const INK_PALETTE: &str =
    "palette \"p\" {\n    ramp \"ink\" \"#000000\" \"#ffffff\" \"#ff0000\"\n}\n";

fn document(name: &str, scene: &str) -> Document {
    let mut files = MemoryFiles::default();
    let document =
        Document::from_text(Path::new(name), format!("{INK_PALETTE}{scene}"), &mut files).unwrap();
    assert_eq!(document.errors(), [] as [String; 0]);
    document
}

#[test]
fn split_layers_tell_anonymous_layers_apart_and_hide_the_others() {
    let scene = concat!(
        "scene \"dup\" palette=\"p\" width=4 height=4 fps=12 duration=\"2f\" background=\"ink.0\" {\n",
        "    layer { rect x=0 y=0 width=1 height=1 paint=\"ink.1\" }\n",
        "    layer { rect x=1 y=0 width=1 height=1 paint=\"ink.1\" }\n",
        "    layer \"fx/glow\" { rect x=0 y=1 width=1 height=1 paint=\"ink.1\" }\n",
        "}\n"
    );
    let document = document("dup.kdl", scene);
    let job = ExportJob {
        kind: ExportKind::PngSequence,
        split_layers: true,
        transparent: false,
        frames: FrameScope::Current(0),
        ..sheet_job("out/dup.png")
    };
    let outputs = px_editor::domain::export::run(&job, &document, &BTreeSet::new()).unwrap();
    let paths: Vec<&Path> = outputs.iter().map(|output| output.path.as_path()).collect();
    assert_eq!(
        paths,
        [
            "out/dup-layer-0-0000.png",
            "out/dup-layer-1-0000.png",
            "out/dup-fx-glow-0000.png",
        ]
        .map(Path::new)
    );
    let distinct: BTreeSet<&[u8]> = outputs
        .iter()
        .map(|output| output.bytes.as_slice())
        .collect();
    assert_eq!(
        distinct.len(),
        outputs.len(),
        "every split output shows a different layer"
    );

    let hidden: BTreeSet<String> = ["fx/glow".to_owned()].into_iter().collect();
    let outputs = px_editor::domain::export::run(&job, &document, &hidden).unwrap();
    let paths: Vec<&Path> = outputs.iter().map(|output| output.path.as_path()).collect();
    assert_eq!(
        paths,
        ["out/dup-layer-0-0000.png", "out/dup-layer-1-0000.png"].map(Path::new)
    );
}

#[test]
fn a_sequence_names_its_files_by_frame_index() {
    let (mut app, mut files, main) = open();
    let sequence = |frame| ExportJob {
        kind: ExportKind::PngSequence,
        frames: FrameScope::Current(frame),
        ..sheet_job("out/room.png")
    };
    app.dispatch(Intent::Export(sequence(3)), main, &mut files);
    app.dispatch(Intent::Export(sequence(4)), main, &mut files);
    let names: Vec<&Path> = files.bytes.keys().map(PathBuf::as_path).collect();
    assert_eq!(
        names,
        ["out/room-0003.png", "out/room-0004.png"].map(Path::new)
    );
}

#[test]
fn an_opaque_export_is_not_reported_as_trimmed() {
    let (mut app, mut files, main) = open();
    app.dispatch(
        Intent::Export(ExportJob {
            trim: true,
            transparent: false,
            ..sheet_job("out/room.png")
        }),
        main,
        &mut files,
    );
    assert_eq!(app.editing().status.text, "Exported 2 files to out");
    assert_eq!(
        png_size(&files.bytes[Path::new("out/room.png")]),
        (u32::from(COLUMNS) * 64, 9 * 48)
    );
    let json = text(&files, "out/room.json");
    assert!(json.contains(r#""trimmed":false"#), "{json}");
    assert!(!json.contains(r#""trimmed":true"#), "{json}");
}
