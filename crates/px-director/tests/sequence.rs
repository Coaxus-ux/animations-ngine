#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use px_core::{Frame, FrameCount, FrameRate, Paint, Palette, Rgb, Size, Tone};
use px_director::{Edge, Player, Sequence, SequenceError, Shot, Transition, TransitionKind};
use px_engine::{
    Bounds, Environment, Geometry, Looping, Param, Scene, SceneSpec, Shape, Style, Timing,
};

const WIDTH: u16 = 8;

/// A scene with a one-pixel mark moving right one pixel per frame, in a palette of four
/// tones scaled by `tint`, so two scenes never share colors.
fn scene(tint: [u8; 3], frames: u32, looping: Looping, size: (u16, u16)) -> Scene {
    let colors: Vec<Rgb> = (1..=4u8)
        .map(|step| Rgb::new(tint[0] * step, tint[1] * step, tint[2] * step))
        .collect();
    let mut palette = Palette::new();
    let ramp = palette.push_ramp(&colors).unwrap();
    let paint = |tone| Paint {
        ramp,
        tone: Tone::new(tone),
    };
    let mark = Shape {
        geometry: Geometry::Rect(Bounds {
            x: Param::Expr(px_expr::compile("t * 4").unwrap()),
            y: Param::Fixed(0.0),
            width: Param::Fixed(1.0),
            height: Param::Fixed(1.0),
        }),
        paint: paint(3),
        style: Style::default(),
    };
    Scene::new(SceneSpec {
        name: "s".to_owned(),
        size: Size::new(size.0, size.1).unwrap(),
        timing: Timing {
            fps: FrameRate::new(4).unwrap(),
            duration: FrameCount::new(frames).unwrap(),
            looping,
        },
        seed: 1,
        palette,
        background: paint(2),
        shapes: vec![mark],
        environment: Environment::default(),
    })
    .unwrap()
}

fn shot(scene: Scene, frames: u32) -> Shot {
    Shot {
        scene,
        frames: FrameCount::new(frames).unwrap(),
        from: Frame::new(0),
        speed: 1.0,
    }
}

fn red() -> Scene {
    scene([40, 0, 0], 8, Looping::Loop, (WIDTH, 4))
}

fn blue() -> Scene {
    scene([0, 0, 40], 8, Looping::Loop, (WIDTH, 4))
}

fn two(kind: TransitionKind, frames: u32) -> Sequence {
    let transition = Transition { kind, frames };
    Sequence::new(
        "q".to_owned(),
        vec![shot(red(), 6), shot(blue(), 6)],
        vec![transition],
    )
    .unwrap()
}

fn shots_at(sequence: &Sequence, frame: u32) -> Vec<u8> {
    let mut player = Player::new(sequence);
    player
        .render(sequence, Frame::new(frame))
        .unwrap()
        .shots()
        .to_vec()
}

#[test]
fn a_cut_switches_shots_and_transitions_overlap_them() {
    let cut = two(TransitionKind::Cut, 0);
    assert_eq!(cut.duration().get(), 12);
    assert!(shots_at(&cut, 5).iter().all(|shot| *shot == 0));
    assert!(shots_at(&cut, 6).iter().all(|shot| *shot == 1));
    assert_eq!(two(TransitionKind::Dither, 3).duration().get(), 9);
}

#[test]
fn a_dither_halfway_mixes_both_shots_with_their_own_colors() {
    let sequence = two(TransitionKind::Dither, 3);
    let mut player = Player::new(&sequence);
    let view = player.render(&sequence, Frame::new(4)).unwrap();
    assert!(view.shots().contains(&0) && view.shots().contains(&1));
    for index in 0..view.paints().len() {
        assert!(
            view.resolve(index).is_ok(),
            "pixel {index} is a color of its own shot"
        );
    }
}

#[test]
fn black_reaches_the_darkest_color_at_its_midpoint() {
    let sequence = two(TransitionKind::Black, 3);
    let mut player = Player::new(&sequence);
    let view = player.render(&sequence, Frame::new(4)).unwrap();
    let darkest = sequence.shots()[1].scene.palette().darkest().unwrap();
    assert!(view.paints().iter().all(|paint| *paint == darkest));
    let before = player.render(&sequence, Frame::new(3)).unwrap();
    assert!(
        before.shots().iter().all(|shot| *shot == 0),
        "the first half fades the first shot"
    );
}

#[test]
fn a_wipe_moves_in_from_its_edge() {
    let sequence = two(TransitionKind::Wipe(Edge::Left), 4);
    let mut covered = Vec::new();
    for frame in 2..=6 {
        let shots = shots_at(&sequence, frame);
        let count: usize = shots.iter().map(|shot| usize::from(*shot == 1)).sum();
        covered.push(count);
        if count > 0 && count < shots.len() {
            assert_eq!(shots[0], 1, "the left edge is covered first");
        }
    }
    assert!(
        covered.windows(2).all(|pair| pair[0] <= pair[1]),
        "{covered:?}"
    );
}

#[test]
fn shots_hold_or_wrap_their_scene_and_start_where_asked() {
    let hash = |sequence: &Sequence, frame| {
        let mut player = Player::new(sequence);
        player.render(sequence, Frame::new(frame)).unwrap().hash()
    };
    let held = scene([40, 0, 0], 2, Looping::Once, (WIDTH, 4));
    let sequence = Sequence::new("q".to_owned(), vec![shot(held, 4)], vec![]).unwrap();
    assert_eq!(
        hash(&sequence, 1),
        hash(&sequence, 3),
        "a scene that ends holds its last frame"
    );
    let wrapped = scene([40, 0, 0], 2, Looping::Loop, (WIDTH, 4));
    let sequence = Sequence::new("q".to_owned(), vec![shot(wrapped, 4)], vec![]).unwrap();
    assert_eq!(
        hash(&sequence, 0),
        hash(&sequence, 2),
        "a looping scene wraps"
    );
    let mut late = shot(red(), 4);
    late.from = Frame::new(1);
    late.speed = 2.0;
    let late = Sequence::new("q".to_owned(), vec![late], vec![]).unwrap();
    let plain = Sequence::new("q".to_owned(), vec![shot(red(), 8)], vec![]).unwrap();
    assert_eq!(hash(&late, 1), hash(&plain, 3), "from 1, at double speed");
}

#[test]
fn sequence_mistakes_are_reported() {
    let new = |shots, transitions| Sequence::new("q".to_owned(), shots, transitions).unwrap_err();
    assert_eq!(new(vec![], vec![]), SequenceError::NoShots);
    let small = scene([40, 0, 0], 8, Looping::Loop, (4, 4));
    assert_eq!(
        new(vec![shot(red(), 4), shot(small, 4)], vec![Transition::CUT]),
        SequenceError::Mismatch {
            shot: 1,
            what: "size"
        }
    );
    let long = Transition {
        kind: TransitionKind::Dither,
        frames: 4,
    };
    assert_eq!(
        new(vec![shot(red(), 4), shot(blue(), 8)], vec![long]),
        SequenceError::TransitionTooLong { shot: 0 }
    );
}
