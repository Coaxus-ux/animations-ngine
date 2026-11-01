#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use px_core::{Frame, hash_cells};
use px_director::{Player, Sequence};
use px_engine::{FrameBuffer, Looping, Scene};
use px_format::{LoadOptions, Work};

fn entries(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir).map_or_else(
        |_| Vec::new(),
        |entries| entries.map(|entry| entry.unwrap().path()).collect(),
    )
}

fn kdl_files(dir: &Path) -> Vec<PathBuf> {
    entries(dir)
        .into_iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "kdl"))
        .collect()
}

fn scene_files(root: &Path) -> Vec<(String, PathBuf)> {
    let repo = root.join("../..");
    let mut files: Vec<(String, PathBuf)> = kdl_files(&repo.join("examples"))
        .into_iter()
        .map(|path| (stem(&path), path))
        .collect();
    for dir in entries(&repo.join("benchmarks"))
        .into_iter()
        .filter(|path| path.is_dir())
    {
        let prefix = stem(&dir);
        files.extend(
            kdl_files(&dir)
                .into_iter()
                .map(|path| (format!("{prefix}-{}", stem(&path)), path)),
        );
    }
    files.sort();
    files
}

fn stem(path: &Path) -> String {
    path.file_stem().unwrap().to_string_lossy().into_owned()
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

fn sequence_hashes(sequence: &Sequence) -> String {
    let mut player = Player::new(sequence);
    let mut hashes = String::new();
    for frame in sequence.duration().frames() {
        let view = player.render(sequence, frame).unwrap();
        assert!(
            (0..view.paints().len()).all(|index| view.resolve(index).is_ok()),
            "{}: frame {} leaves its shots' palettes",
            sequence.name(),
            frame.index()
        );
        writeln!(hashes, "{} {:016x}", frame.index(), view.hash()).unwrap();
    }
    hashes
}

#[test]
fn every_example_and_benchmark_matches_its_golden_hashes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let files = scene_files(root);
    assert_ne!(files.len(), 0, "no example scenes found");
    for (name, file) in files {
        let actual = match px_format::load_work_file(&file, LoadOptions::default()).unwrap() {
            Work::Scene(loaded) => frame_hashes(&loaded.scene),
            Work::Sequence(loaded) => sequence_hashes(&loaded.sequence),
        };
        let golden = root.join(format!("tests/golden/{name}.txt"));
        if std::env::var_os("PX_BLESS").is_some() {
            std::fs::write(&golden, &actual).unwrap();
        }
        let expected = std::fs::read_to_string(&golden).unwrap_or_default();
        assert_eq!(
            actual, expected,
            "{name}: golden hashes changed; rerun with PX_BLESS=1 if intended"
        );
    }
}
