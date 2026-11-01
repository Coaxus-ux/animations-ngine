#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use std::path::{Path, PathBuf};
use std::process::Command;

use px_core::Rgb;
use px_director::Player;
use px_engine::FrameBuffer;
use px_export::{Scale, resolve_rgb};
use px_format::{LoadOptions, Work};

const EXAMPLE: &str = "campfire-forest";

fn export(example: &Path, jobs: u32, out: &Path) -> Vec<u8> {
    let status = Command::new(env!("CARGO_BIN_EXE_pxe"))
        .arg("export")
        .arg(example)
        .args(["--scale", "1", "--jobs", &jobs.to_string(), "--out"])
        .arg(out)
        .output()
        .unwrap();
    assert!(status.status.success(), "{status:?}");
    std::fs::read(out).unwrap()
}

fn decoded_frames(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = options.read_info(bytes).unwrap();
    let width = usize::from(decoder.width());
    let mut screen = vec![0; width * usize::from(decoder.height()) * 3];
    let mut frames = Vec::new();
    while let Some(frame) = decoder.read_next_frame().unwrap() {
        let (left, top) = (usize::from(frame.left), usize::from(frame.top));
        for (i, pixel) in frame.buffer.as_chunks::<4>().0.iter().enumerate() {
            if pixel[3] == 0 {
                continue;
            }
            let (x, y) = (
                left + i % usize::from(frame.width),
                top + i / usize::from(frame.width),
            );
            let at = (y * width + x) * 3;
            screen[at..at + 3].copy_from_slice(&pixel[..3]);
        }
        frames.push(screen.clone());
    }
    frames
}

fn rendered_frames(example: &Path) -> Vec<Vec<u8>> {
    let scale = Scale::new(1).unwrap();
    match px_format::load_work_file(example, LoadOptions::default()).unwrap() {
        Work::Scene(loaded) => {
            let scene = &loaded.scene;
            let mut buffer = FrameBuffer::new(scene);
            scene
                .timing()
                .duration
                .frames()
                .map(|frame| {
                    let view = buffer.render(scene, frame).unwrap();
                    let mut rgb = vec![0; view.cells().len() * 3];
                    resolve_rgb(&view, scene.palette(), scale, &mut rgb).unwrap();
                    rgb
                })
                .collect()
        }
        Work::Sequence(loaded) => {
            let sequence = &loaded.sequence;
            let mut player = Player::new(sequence);
            sequence
                .duration()
                .frames()
                .map(|frame| {
                    let view = player.render(sequence, frame).unwrap();
                    (0..view.paints().len())
                        .flat_map(|i| {
                            let Rgb { r, g, b } = view.resolve(i).unwrap();
                            [r, g, b]
                        })
                        .collect()
                })
                .collect()
        }
    }
}

#[test]
fn export_is_independent_of_jobs_and_decodes_to_the_rendered_frames() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let example = repo.join(format!("examples/{EXAMPLE}.kdl"));
    let out: PathBuf = std::env::temp_dir().join(format!("pxe-gif-export-{}", std::process::id()));
    std::fs::create_dir_all(&out).unwrap();
    let serial = export(&example, 1, &out.join("serial.gif"));
    let parallel = export(&example, 3, &out.join("parallel.gif"));
    std::fs::remove_dir_all(&out).unwrap();
    assert!(
        serial == parallel,
        "{EXAMPLE}: bytes differ between --jobs 1 and --jobs 3"
    );
    let decoded = decoded_frames(&serial);
    let rendered = rendered_frames(&example);
    assert_eq!(decoded.len(), rendered.len());
    for (index, (decoded, rendered)) in decoded.iter().zip(&rendered).enumerate() {
        assert!(
            decoded == rendered,
            "{EXAMPLE}: frame {index} decodes differently"
        );
    }
}
