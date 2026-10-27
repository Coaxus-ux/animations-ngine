//! Golden-hash test for the example scene: indexed-buffer hashes per frame must match
//! `tests/golden/hello.txt` on every platform. Set `PX_BLESS=1` to rewrite the file
//! after an intended change, and review the diff.
#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use std::fmt::Write as _;
use std::path::Path;

use px_core::{Frame, hash_cells};
use px_engine::{FrameBuffer, Looping};

#[test]
fn hello_example_matches_golden_hashes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(root.join("../../examples/hello.kdl")).unwrap();
    let scene = px_format::load_scene(&source, None).unwrap().scene;
    let mut buffer = FrameBuffer::new(&scene);

    let mut actual = String::new();
    for frame in scene.timing().duration.frames() {
        let view = buffer.render(&scene, frame).unwrap();
        assert!(
            view.cells()
                .iter()
                .all(|paint| scene.palette().contains(*paint)),
            "frame {} leaves the palette",
            frame.index()
        );
        writeln!(
            actual,
            "{} {:016x}",
            frame.index(),
            hash_cells(view.cells())
        )
        .unwrap();
    }

    if scene.timing().looping == Looping::Loop {
        let first = hash_cells(buffer.render(&scene, Frame::new(0)).unwrap().cells());
        let wrapped = hash_cells(
            buffer
                .render(&scene, Frame::new(scene.timing().duration.get()))
                .unwrap()
                .cells(),
        );
        assert_eq!(first, wrapped, "the loop has a seam");
    }

    let golden = root.join("tests/golden/hello.txt");
    if std::env::var_os("PX_BLESS").is_some() {
        std::fs::write(&golden, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(&golden).unwrap_or_default();
    assert_eq!(
        actual, expected,
        "golden hashes changed; rerun with PX_BLESS=1 if intended"
    );
}
