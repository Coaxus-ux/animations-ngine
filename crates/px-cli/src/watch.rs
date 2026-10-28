use std::fmt::Write as _;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use px_core::Frame;
use px_engine::{FrameBuffer, Scene};
use px_export::Scale;

use crate::args::Input;
use crate::file_watch::FileWatch;
use crate::playback::Playback;
use crate::preview::{Control, Preview};
use crate::source;

const POLL: Duration = Duration::from_millis(250);
const TIMING_SAMPLE_FRAMES: u32 = 120;

struct Live {
    scene: Scene,
    buffer: FrameBuffer,
    frame_ms: f64,
}

impl Live {
    fn new(scene: Scene) -> Result<Self> {
        let mut buffer = FrameBuffer::new(&scene);
        let frame_ms = average_frame_ms(&scene, &mut buffer)?;
        Ok(Self {
            scene,
            buffer,
            frame_ms,
        })
    }
}

pub(crate) fn watch(input: &Input, scale: Scale) -> Result<()> {
    let mut file = FileWatch::new(&input.file);
    let mut live = wait_for_valid_scene(input, &mut file)?;
    let mut preview = Preview::open(live.scene.size(), scale, live.scene.name())?;
    let mut playback = Playback::default();
    let mut last_poll = Instant::now();
    let mut last_tick = Instant::now();
    println!(
        "watching {} (space: pause, ←/→: step, r: restart, esc: quit)",
        input.file.display()
    );

    while preview.is_open() {
        let now = Instant::now();
        playback.advance(now - last_tick);
        last_tick = now;

        if now - last_poll >= POLL {
            last_poll = now;
            if file.changed() {
                reload(input, &mut live, &mut preview, scale)?;
            }
        }
        for control in preview.controls() {
            match control {
                Control::TogglePause => playback.toggle_pause(),
                Control::StepBack => playback.step(-1, live.scene.timing()),
                Control::StepForward => playback.step(1, live.scene.timing()),
                Control::Restart => playback.restart(),
                Control::Quit => return Ok(()),
            }
        }
        let frame = playback.frame(live.scene.timing());
        let view = live.buffer.render(&live.scene, frame)?;
        preview.show(&view, live.scene.palette())?;
        preview.set_title(&title(&live, frame, playback.is_paused()));
    }
    Ok(())
}

fn wait_for_valid_scene(input: &Input, file: &mut FileWatch) -> Result<Live> {
    loop {
        if file.changed() {
            match source::load(input) {
                Ok(scene) => return Live::new(scene),
                Err(error) => eprintln!("{error:#}; waiting for the file to change…"),
            }
        }
        thread::sleep(POLL);
    }
}

fn reload(input: &Input, live: &mut Live, preview: &mut Preview, scale: Scale) -> Result<()> {
    match source::load(input) {
        Ok(scene) => {
            *live = Live::new(scene)?;
            if preview.size() != live.scene.size() {
                *preview = Preview::open(live.scene.size(), scale, live.scene.name())?;
            }
            println!(
                "reloaded {} ({} shapes, {:.2} ms/frame)",
                input.file.display(),
                live.scene.shapes().len(),
                live.frame_ms
            );
        }
        Err(error) => eprintln!("{error:#}; keeping the last valid version"),
    }
    Ok(())
}

fn average_frame_ms(scene: &Scene, buffer: &mut FrameBuffer) -> Result<f64> {
    let frames = scene.timing().duration.get().min(TIMING_SAMPLE_FRAMES);
    let started = Instant::now();
    for index in 0..frames {
        buffer.render(scene, Frame::new(index))?;
    }
    Ok(started.elapsed().as_secs_f64() * 1000.0 / f64::from(frames))
}

fn title(live: &Live, frame: Frame, paused: bool) -> String {
    let timing = live.scene.timing();
    let mut title = format!(
        "{} — frame {}/{} — {} fps — {:.2} ms/frame",
        live.scene.name(),
        frame.index() + 1,
        timing.duration.get(),
        timing.fps.get(),
        live.frame_ms
    );
    if paused {
        let _ = write!(title, " — paused");
    }
    title
}
