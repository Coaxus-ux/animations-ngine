//! Golden-hash test for every scene in `examples/`: indexed-buffer hashes per frame must
//! match `tests/golden/<name>.txt` on every platform. Set `PX_BLESS=1` to rewrite the files
//! after an intended change, and review the diff.
#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use px_core::{Frame, hash_cells};
use px_engine::{FrameBuffer, Looping, Scene};
use px_format::LoadOptions;

fn examples(root: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(root.join("../../examples"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "kdl"))
        .collect();
    files.sort();
    files
}

fn frame_hashes(scene: &Scene) -> String {
    let mut buffer = FrameBuffer::new(scene);
    let mut hashes = String::new();
    for frame in scene.timing().duration.frames() {
        let view = buffer.render(scene, frame).unwrap();
        assert!(
            view.cells()
                .iter()
                .all(|paint| scene.palette().contains(*paint)),
            "{}: frame {} leaves the palette",
            scene.name(),
            frame.index()
        );
        writeln!(
            hashes,
            "{} {:016x}",
            frame.index(),
            hash_cells(view.cells())
        )
        .unwrap();
    }
    if scene.timing().looping == Looping::Loop {
        let first = hash_cells(buffer.render(scene, Frame::new(0)).unwrap().cells());
        let end = Frame::new(scene.timing().duration.get());
        let wrapped = hash_cells(buffer.render(scene, end).unwrap().cells());
        assert_eq!(first, wrapped, "{}: the loop has a seam", scene.name());
    }
    hashes
}

#[test]
fn every_example_matches_its_golden_hashes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let files = examples(root);
    assert_ne!(files.len(), 0, "no example scenes found");
    for file in files {
        let source = std::fs::read_to_string(&file).unwrap();
        let scene = px_format::load_scene(&source, LoadOptions::default())
            .unwrap()
            .scene;
        let actual = frame_hashes(&scene);
        let stem = file.file_stem().unwrap().to_string_lossy();
        let golden = root.join(format!("tests/golden/{stem}.txt"));
        if std::env::var_os("PX_BLESS").is_some() {
            std::fs::write(&golden, &actual).unwrap();
        }
        let expected = std::fs::read_to_string(&golden).unwrap_or_default();
        assert_eq!(
            actual, expected,
            "{stem}: golden hashes changed; rerun with PX_BLESS=1 if intended"
        );
    }
}
