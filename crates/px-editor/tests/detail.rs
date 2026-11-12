#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use common::{SIZE, open};
use px_core::Frame;
use px_editor::app::{DocumentId, Intent, ViewState};
use px_editor::views::RenderCache;
use px_engine::{Detail, FrameBuffer};
use px_ui::Input;

const LATE_FRAMES: u64 = 3;

#[test]
fn the_canvas_shows_the_document_at_the_view_detail() {
    let (app, _, _) = open();
    let document = app.document().unwrap();
    let view = ViewState {
        detail: Detail::whole(2),
        ..ViewState::default()
    };
    let mut cache = RenderCache::new();
    cache.update(DocumentId(0), document, &view, false);
    let image = cache.image().unwrap();
    assert_eq!(image.canvas, (64, 48), "tools keep working in scene pixels");
    assert_eq!(image.size, (128, 96));
    assert_eq!(image.pixel_of(10, 7), Some((21, 15)));
    assert_eq!(image.pixel_of(64, 0), None);
    let cell = image.cells[15 * 128 + 21];
    assert_eq!(cache.paint_at(10, 7), Some(cell));
}

#[test]
fn playback_reuses_rendered_frames_without_changing_them() {
    let (app, _, _) = open();
    let document = app.document().unwrap();
    let scene = document.scene().unwrap();
    let mut buffer = FrameBuffer::new(scene);
    let mut cache = RenderCache::new();
    for round in 0..2 {
        for frame in 0..scene.timing().duration.get() {
            let view = ViewState {
                frame,
                ..ViewState::default()
            };
            cache.update(DocumentId(0), document, &view, true);
            let fresh = buffer.render(scene, Frame::new(frame)).unwrap();
            assert_eq!(
                cache.image().unwrap().cells,
                fresh.cells(),
                "round {round}, frame {frame}"
            );
        }
    }
}

#[test]
fn playback_keeps_time_when_the_editor_falls_behind() {
    let (mut app, mut files, main) = open();
    let timing = app.document().unwrap().scene().unwrap().timing();
    let fps = u64::from(timing.fps.get());
    app.dispatch(Intent::TogglePlay, main, &mut files);
    let late = Input {
        time_ms: LATE_FRAMES * 1000 / fps + 1,
        ..Input::default()
    };
    app.frame(main, &late, SIZE, &mut files);
    let expected = u32::try_from(LATE_FRAMES).unwrap() % timing.duration.get();
    assert_eq!(app.window(main).unwrap().view.frame, expected);
}
