use std::time::Duration;

use px_core::{FrameCount, FrameRate};
use px_engine::{Looping, Timing};

use super::Playback;

fn timing(frames: u32, looping: Looping) -> Timing {
    Timing {
        fps: FrameRate::new(12).unwrap(),
        duration: FrameCount::new(frames).unwrap(),
        looping,
    }
}

fn at(seconds: f64, timing: Timing) -> u32 {
    let mut playback = Playback::default();
    playback.advance(Duration::from_secs_f64(seconds));
    playback.frame(timing).index()
}

#[test]
fn looping_scenes_wrap_and_one_shots_hold_the_last_frame() {
    assert_eq!(at(0.5, timing(24, Looping::Loop)), 6);
    assert_eq!(at(2.5, timing(24, Looping::Loop)), 6);
    assert_eq!(at(2.5, timing(24, Looping::Once)), 23);
}

#[test]
fn pause_freezes_time_and_steps_move_one_frame() {
    let timing = timing(24, Looping::Loop);
    let mut playback = Playback::default();
    playback.advance(Duration::from_millis(250));
    playback.toggle_pause();
    playback.advance(Duration::from_secs(5));
    assert_eq!(playback.frame(timing).index(), 3);
    playback.step(1, timing);
    assert_eq!(playback.frame(timing).index(), 4);
    playback.step(-2, timing);
    assert_eq!(playback.frame(timing).index(), 2);
    playback.step(-9, timing);
    assert_eq!(playback.frame(timing).index(), 0);
    playback.restart();
    playback.toggle_pause();
    assert!(!playback.is_paused());
    assert_eq!(playback.frame(timing).index(), 0);
}
