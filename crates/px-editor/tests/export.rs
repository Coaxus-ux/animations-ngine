//! Exports driven headless: sprite sheets, PNGs and GIFs land in the in-memory store.
#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use common::{layer, open, stroke};
use px_core::{Frame, Rgb};
use px_editor::app::{App, DialogKind, Intent, PickPurpose, Request};
use px_editor::domain::Document;
use px_editor::domain::export::{
    ExportJob, ExportKind, FRAME_INDEX_DIGITS, FrameScope, sequence_name,
};
use px_editor::domain::frames::render_frames;
use px_editor::tools::ToolKind;
use px_engine::FrameBuffer;
use px_export::{Padding, SheetLayout};

const FRAMES: u32 = 18;
const WIDTH: u32 = 64;
const HEIGHT: u32 = 48;

struct Png {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl Png {
    fn decode(bytes: &[u8]) -> Self {
        let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
            .read_info()
            .unwrap();
        let mut rgba = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut rgba).unwrap();
        assert_eq!(info.color_type, png::ColorType::Rgba);
        rgba.truncate(info.buffer_size());
        Self {
            width: info.width,
            height: info.height,
            rgba,
        }
    }

    fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let start = index(x, y, self.width) * 4;
        self.rgba[start..start + 4].try_into().unwrap()
    }
}

fn index(x: u32, y: u32, width: u32) -> usize {
    usize::try_from(y * width + x).unwrap()
}

fn job(kind: ExportKind, transparent: bool, frames: FrameScope, out: &str) -> ExportJob {
    ExportJob {
        kind,
        scale: 1,
        transparent,
        frames,
        out: PathBuf::from(out),
    }
}

fn sheet(columns: u16, json: bool) -> ExportKind {
    ExportKind::Sheet {
        layout: SheetLayout::Rows { columns },
        padding: Padding::default(),
        json,
    }
}

/// A canvas pixel the opaque render leaves exactly as the scene's background paint.
fn background_pixel(app: &App) -> ((u32, u32), Rgb) {
    let scene = app.document().unwrap().scene().unwrap();
    let mut buffer = FrameBuffer::new(scene);
    let view = buffer.render(scene, Frame::new(0)).unwrap();
    let index = view
        .cells()
        .iter()
        .position(|cell| *cell == scene.background())
        .unwrap();
    let index = u32::try_from(index).unwrap();
    let color = scene.palette().resolve(scene.background()).unwrap();
    ((index % WIDTH, index / WIDTH), color)
}

#[test]
fn a_sprite_sheet_is_a_transparent_rgba_png_with_aseprite_json() {
    let (mut app, mut files, main) = open();
    let ((x, y), _) = background_pixel(&app);
    app.dispatch(
        Intent::Export(job(sheet(2, true), true, FrameScope::All, "out/room.png")),
        main,
        &mut files,
    );
    assert_eq!(app.editing().status, "Exported 2 files to out");
    let image = Png::decode(&files.bytes[Path::new("out/room.png")]);
    assert_eq!((image.width, image.height), (2 * WIDTH, 9 * HEIGHT));
    assert_eq!(image.pixel(x, y)[3], 0);
    let json = String::from_utf8(files.bytes[Path::new("out/room.json")].clone()).unwrap();
    assert!(json.contains("\"frames\""));
    assert!(json.contains(&format!("\"pixel-room {}.kdl\"", FRAMES - 1)));
    assert!(!json.contains(&format!("\"pixel-room {FRAMES}.kdl\"")));
    assert!(json.contains("\"cat\""));
    assert!(json.contains("\"duration\":83"), "{json}");
}

#[test]
fn keyed_render_keeps_background_transparent_under_lighting_and_keeps_painted_background_paint_opaque()
 {
    let (mut app, mut files, main) = open();
    let ((x, y), _) = background_pixel(&app);
    let at = index(x, y, WIDTH);
    let none = BTreeSet::new();
    let before =
        render_frames(app.document().unwrap(), &none, true, FrameScope::Current(0)).unwrap();
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].pixels[at], None);
    let background = app.document().unwrap().scene().unwrap().background();
    let cat = layer(&app, "cat");
    app.dispatch(Intent::SelectLayer(cat), main, &mut files);
    app.dispatch(Intent::Tool(ToolKind::Pencil), main, &mut files);
    app.dispatch(Intent::Swatch(background), main, &mut files);
    assert_eq!(app.editing().primary, Some(background));
    stroke(&mut app, &mut files, main, &[(3, 2)]);
    let painted = index(3, 2, WIDTH);
    let after =
        render_frames(app.document().unwrap(), &none, true, FrameScope::Current(0)).unwrap();
    assert!(after[0].pixels[painted].is_some());
    assert_eq!(after[0].pixels[at], None);
}

#[test]
fn an_opaque_sheet_keeps_the_background() {
    let (mut app, mut files, main) = open();
    let ((x, y), color) = background_pixel(&app);
    app.dispatch(
        Intent::Export(job(sheet(2, false), false, FrameScope::All, "out/room.png")),
        main,
        &mut files,
    );
    assert_eq!(app.editing().status, "Exported 1 file to out");
    assert!(!files.bytes.contains_key(Path::new("out/room.json")));
    let image = Png::decode(&files.bytes[Path::new("out/room.png")]);
    assert_eq!(image.pixel(x, y), [color.r, color.g, color.b, 255]);
}

#[test]
fn the_png_sequence_names_frames_in_order() {
    let (mut app, mut files, main) = open();
    app.dispatch(
        Intent::Export(job(
            ExportKind::PngSequence,
            true,
            FrameScope::All,
            "out/room.png",
        )),
        main,
        &mut files,
    );
    let names: Vec<&Path> = files.bytes.keys().map(PathBuf::as_path).collect();
    let expected: Vec<PathBuf> = (0..FRAMES)
        .map(|index| PathBuf::from(format!("out/room-{index:04}.png")))
        .collect();
    assert_eq!(
        names,
        expected.iter().map(PathBuf::as_path).collect::<Vec<_>>()
    );
    assert_eq!(
        app.editing().status,
        format!("Exported {FRAMES} files to out")
    );
    let first = Png::decode(&files.bytes[Path::new("out/room-0000.png")]);
    assert_eq!((first.width, first.height), (WIDTH, HEIGHT));
}

#[test]
fn sequence_names_sort_for_every_frame_count() {
    let count = usize::try_from(px_core::MAX_FRAMES).unwrap();
    assert_eq!(FRAME_INDEX_DIGITS, (count - 1).to_string().len());
    let names: Vec<String> = (0..count)
        .map(|index| sequence_name("frame", index))
        .collect();
    assert_eq!(names[0], "frame-0000.png");
    assert_eq!(names[1000], "frame-1000.png");
    assert!(names.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn a_full_palette_reports_a_clear_transparent_export_error() {
    let mut ramps = String::new();
    for ramp in 0..px_core::MAX_RAMPS {
        writeln!(ramps, "    ramp \"r{ramp}\" \"#000000\" \"#ffffff\"").unwrap();
    }
    let text = format!(
        "palette \"full\" {{\n{ramps}}}\nscene \"full\" palette=\"full\" width=4 height=4 fps=12 duration=\"1f\" background=\"r0.0\" {{\n}}\n"
    );
    let mut files = common::MemoryFiles::default();
    let document = Document::from_text(Path::new("full.kdl"), text, &mut files).unwrap();
    assert_eq!(document.errors(), [] as [String; 0]);
    let none = BTreeSet::new();
    let error = render_frames(&document, &none, true, FrameScope::Current(0)).unwrap_err();
    assert!(error.contains("transparent export"), "{error}");
    assert!(error.contains("256 ramps"), "{error}");
    assert!(!error.contains("palette error"), "{error}");
    assert!(render_frames(&document, &none, false, FrameScope::Current(0)).is_ok());
}

#[test]
fn the_current_frame_exports_as_png_and_the_scene_as_gif() {
    let (mut app, mut files, main) = open();
    app.dispatch(
        Intent::Export(job(
            ExportKind::Png,
            true,
            FrameScope::Current(4),
            "out/frame.png",
        )),
        main,
        &mut files,
    );
    let png = &files.bytes[Path::new("out/frame.png")];
    assert_eq!(&png[..4], &[0x89, b'P', b'N', b'G']);
    app.dispatch(
        Intent::Export(job(ExportKind::Gif, true, FrameScope::All, "out/room.gif")),
        main,
        &mut files,
    );
    let gif = &files.bytes[Path::new("out/room.gif")];
    assert_eq!(&gif[..6], b"GIF89a");
    assert_eq!(app.editing().status, "Exported 1 file to out");
}

#[test]
fn a_png_of_every_frame_is_refused_instead_of_exporting_frame_zero() {
    let (mut app, mut files, main) = open();
    app.dispatch(
        Intent::Export(job(ExportKind::Png, true, FrameScope::All, "out/frame.png")),
        main,
        &mut files,
    );
    assert_eq!(app.editing().status, "PNG exports one frame: pick Current");
    assert!(files.bytes.is_empty());
}

#[test]
fn a_gif_exports_only_the_frames_in_scope() {
    let (mut app, mut files, main) = open();
    app.dispatch(
        Intent::Export(job(ExportKind::Gif, false, FrameScope::All, "out/all.gif")),
        main,
        &mut files,
    );
    app.dispatch(
        Intent::Export(job(
            ExportKind::Gif,
            false,
            FrameScope::Current(4),
            "out/one.gif",
        )),
        main,
        &mut files,
    );
    let all = &files.bytes[Path::new("out/all.gif")];
    let one = &files.bytes[Path::new("out/one.gif")];
    assert_eq!(&one[..6], b"GIF89a");
    assert!(one.len() < all.len(), "{} < {}", one.len(), all.len());
}

#[test]
fn hidden_layers_stay_out_of_the_export() {
    let (mut app, mut files, main) = open();
    let shown = job(
        ExportKind::Png,
        false,
        FrameScope::Current(0),
        "out/frame.png",
    );
    app.dispatch(Intent::Export(shown.clone()), main, &mut files);
    let with_plant = files.bytes[Path::new("out/frame.png")].clone();
    app.dispatch(Intent::ToggleHidden("plant".to_owned()), main, &mut files);
    app.dispatch(Intent::Export(shown), main, &mut files);
    let without_plant = &files.bytes[Path::new("out/frame.png")];
    assert_ne!(&with_plant, without_plant);

    let document = app.document().unwrap();
    let hidden: BTreeSet<String> = ["plant".to_owned()].into_iter().collect();
    let visible = render_frames(document, &BTreeSet::new(), false, FrameScope::Current(0)).unwrap();
    let hidden = render_frames(document, &hidden, false, FrameScope::Current(0)).unwrap();
    let leaf = index(46, 28, WIDTH);
    assert_ne!(visible[0].pixels[leaf], hidden[0].pixels[leaf]);
}

#[test]
fn repeat_export_targets_the_active_document() {
    let (mut app, mut files, main) = open();
    app.dispatch(Intent::RepeatExport, main, &mut files);
    assert_eq!(app.editing().status, "Nothing exported yet");
    app.dispatch(
        Intent::Export(job(
            ExportKind::Png,
            true,
            FrameScope::Current(0),
            "out/a.png",
        )),
        main,
        &mut files,
    );
    files.bytes.clear();
    app.dispatch(Intent::RepeatExport, main, &mut files);
    assert_eq!(
        files.bytes.keys().collect::<Vec<_>>(),
        [Path::new("out/room.png")]
    );
    assert_eq!(app.editing().status, "Exported 1 file to out");

    files
        .files
        .insert(PathBuf::from("cat.kdl"), common::example("pixel-room.kdl"));
    app.open(Path::new("cat.kdl"), None, &mut files).unwrap();
    files.bytes.clear();
    app.dispatch(Intent::RepeatExport, main, &mut files);
    assert_eq!(
        files.bytes.keys().collect::<Vec<_>>(),
        [Path::new("out/cat.png")]
    );

    app.dispatch(
        Intent::NewDocument(px_editor::domain::NewSpec::default()),
        main,
        &mut files,
    );
    assert!(app.document().unwrap().is_untitled());
    files.bytes.clear();
    app.dispatch(Intent::RepeatExport, main, &mut files);
    assert!(files.bytes.is_empty());
    let dialog = app.window(main).unwrap().dialog.clone().unwrap();
    assert_eq!(dialog.form.choices["kind"], ExportKind::Png.index());
    assert_eq!(dialog.form.texts["out"], "");
}

#[test]
fn an_export_with_an_empty_path_requests_a_native_save_dialog() {
    let (mut app, mut files, main) = open();
    app.take_requests();
    app.dispatch(
        Intent::Export(job(ExportKind::Gif, true, FrameScope::All, "")),
        main,
        &mut files,
    );
    assert!(files.bytes.is_empty());
    let requests = app.take_requests();
    assert!(requests.iter().any(|request| matches!(
        request,
        Request::PickFile {
            purpose: PickPurpose::ExportOut,
            name: Some(name),
            ..
        } if name == "room.gif"
    )));
}

#[test]
fn a_picked_output_path_lands_in_the_open_dialog() {
    let (mut app, mut files, main) = open();
    app.dispatch(Intent::ShowDialog(DialogKind::Export), main, &mut files);
    let dialog = app.window(main).unwrap().dialog.clone().unwrap();
    assert_eq!(dialog.form.texts["out"], "room.png");
    app.dispatch(
        Intent::Picked {
            purpose: PickPurpose::ExportOut,
            path: Some(PathBuf::from("out/sheet.png")),
        },
        main,
        &mut files,
    );
    let dialog = app.window(main).unwrap().dialog.clone().unwrap();
    assert_eq!(dialog.form.texts["out"], "out/sheet.png");
}
