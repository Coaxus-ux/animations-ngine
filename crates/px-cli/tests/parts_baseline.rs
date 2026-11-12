#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use px_core::hash_cells;
use px_engine::FrameBuffer;
use px_format::LoadOptions;
use px_format::edit::{Source, id_of};

fn kdl_files(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            kdl_files(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "kdl") {
            found.push(path);
        }
    }
}

#[test]
#[ignore = "writes target/parts-baseline.txt, compared by hand while migrating examples"]
fn hash_every_part_scene_on_its_own() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    kdl_files(&root.join("examples/parts"), &mut files);
    files.sort();
    let mut report = String::new();
    for path in files {
        let source = Source::parse(std::fs::read_to_string(&path).unwrap()).unwrap();
        let scenes: Vec<String> = source
            .nodes()
            .iter()
            .filter(|node| node.name().value() == "scene")
            .filter_map(id_of)
            .map(str::to_owned)
            .collect();
        for name in scenes {
            let options = LoadOptions {
                scene: Some(&name),
                ..LoadOptions::default()
            };
            let scene = px_format::load_file(&path, options).unwrap().scene;
            let mut buffer = FrameBuffer::new(&scene);
            let mut hashes = 0_u64;
            for frame in scene.timing().duration.frames() {
                let cells = hash_cells(buffer.render(&scene, frame).unwrap().cells());
                hashes = hashes.rotate_left(7) ^ cells;
            }
            writeln!(report, "{name} {hashes:016x}").unwrap();
        }
    }
    std::fs::write(root.join("target/parts-baseline.txt"), report).unwrap();
}
