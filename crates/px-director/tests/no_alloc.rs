//! Certifies that `Player::render` performs no heap activity after setup, in shots and
//! in every kind of transition. Run alone: `--test-threads=1`.
#![allow(
    unsafe_code,
    reason = "test-only counting allocator, see DECISIONS.md D-002"
)]
#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

#[path = "../../px-engine/tests/support/counting_alloc.rs"]
mod counting_alloc;

use counting_alloc::assert_no_heap_activity;
use px_core::{Frame, FrameCount, FrameRate, Paint, Palette, Rgb, Size, Tone};
use px_director::{Edge, Player, Sequence, Shot, Transition, TransitionKind};
use px_engine::{Environment, Looping, Scene, SceneSpec, Timing};

fn scene(tint: u8) -> Scene {
    let colors: Vec<Rgb> = (1..=4u8)
        .map(|step| Rgb::new(tint * step, 10, 10))
        .collect();
    let mut palette = Palette::new();
    let ramp = palette.push_ramp(&colors).unwrap();
    Scene::new(SceneSpec {
        name: "s".to_owned(),
        size: Size::new(16, 8).unwrap(),
        timing: Timing {
            fps: FrameRate::new(4).unwrap(),
            duration: FrameCount::new(8).unwrap(),
            looping: Looping::Loop,
        },
        seed: 1,
        palette,
        background: Paint {
            ramp,
            tone: Tone::new(2),
        },
        shapes: vec![],
        environment: Environment::default(),
    })
    .unwrap()
}

#[test]
fn rendering_a_sequence_performs_no_heap_activity() {
    let kinds = [
        TransitionKind::Cut,
        TransitionKind::Dither,
        TransitionKind::Black,
        TransitionKind::White,
        TransitionKind::Wipe(Edge::Bottom),
    ];
    let shots = (0..=kinds.len())
        .map(|index| Shot {
            scene: scene(u8::try_from(index).unwrap() * 8 + 8),
            frames: FrameCount::new(8).unwrap(),
            from: Frame::new(0),
            speed: 1.0,
        })
        .collect();
    let transitions = kinds.map(|kind| Transition { kind, frames: 3 }).to_vec();
    let sequence = Sequence::new("q".to_owned(), shots, transitions).unwrap();
    let mut player = Player::new(&sequence);
    assert_no_heap_activity(|| {
        for frame in sequence.duration().frames() {
            player.render(&sequence, frame).unwrap();
        }
    });
}
