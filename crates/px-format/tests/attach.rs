#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use px_core::{Frame, Paint};
use px_engine::{FrameBuffer, Scene};
use px_format::{LoadOptions, load_scene};

const PALETTE: &str = r##"palette "p" { ramp "sky" "#000000" "#111111"; ramp "ink" "#ffffff"; ramp "red" "#ff0000" }"##;

const MOVER: &str = r#"scene "mover" palette="p" width=16 height=4 fps=4 duration="2s" loop=#true background="sky" {
    rect "mark" x="=t * 4" y=1 width=1 height=1 paint="ink"
}"#;

fn options() -> LoadOptions<'static> {
    LoadOptions {
        scene: Some("main"),
        ..LoadOptions::default()
    }
}

fn main_scene(body: &str) -> String {
    format!(
        "{PALETTE}\n{MOVER}\nscene \"main\" palette=\"p\" width=16 height=4 fps=4 duration=\"2s\" loop=#true background=\"sky\" {{\n{body}\n}}\n"
    )
}

fn load(body: &str) -> Scene {
    load_scene(&main_scene(body), options()).unwrap().scene
}

fn error(body: &str) -> String {
    load_scene(&main_scene(body), options())
        .unwrap_err()
        .diagnostics[0]
        .message
        .clone()
}

fn red_columns(scene: &Scene, frame: u32) -> Vec<usize> {
    let mut buffer = FrameBuffer::new(scene);
    let view = buffer.render(scene, Frame::new(frame)).unwrap();
    let red: Paint = scene.shapes().last().unwrap().paint;
    (0..16).filter(|x| view.cells()[16 + x] == red).collect()
}

#[test]
fn an_attached_shape_moves_with_the_object_inside_a_placed_scene() {
    let scene = load(
        "use-scene \"mover\" id=\"m\" x=2\nrect attach=\"m.mark\" x=1 y=0 width=1 height=1 paint=\"red\"",
    );
    assert_eq!(red_columns(&scene, 0), [3]);
    assert_eq!(
        red_columns(&scene, 3),
        [6],
        "the mark moved 3 px, so did the rect"
    );
}

#[test]
fn an_attached_object_turns_around_with_a_flipped_placement() {
    let scene = load(
        "use-scene \"mover\" id=\"m\" x=12 flip=#true\nrect attach=\"m.mark\" x=2 y=0 width=1 height=1 paint=\"red\"",
    );
    assert_eq!(red_columns(&scene, 0), [9]);
}

#[test]
fn lights_and_placed_scenes_can_be_attached() {
    let scene = load(
        "use-scene \"mover\" id=\"m\" x=2\nuse-scene \"mover\" attach=\"m.mark\" y=1\nlight-point attach=\"m\" x=1 y=1 reach=4",
    );
    let world = scene.environment();
    let light_anchor = world.lights[0].anchor.as_ref().unwrap();
    assert_eq!(light_anchor.instance, Some(0));
    assert_eq!(
        world.instances[1].anchor.as_ref().unwrap().instance,
        Some(0)
    );
}

#[test]
fn attach_mistakes_are_explained() {
    assert!(
        error("rect attach=\"m.mark\" x=0 y=0 width=1 height=1 paint=\"red\"\nuse-scene \"mover\" id=\"m\"")
            .contains("attach only to objects written before"),
    );
    assert!(
        error("use-scene \"mover\" id=\"m\" count=2\nrect attach=\"m.mark\" x=0 y=0 width=1 height=1 paint=\"red\"")
            .contains("places 2 copies"),
    );
    assert!(
        error("use-scene \"mover\" id=\"m\"\nrect attach=\"m.mrak\" x=0 y=0 width=1 height=1 paint=\"red\"")
            .contains("Did you mean `mark`?"),
    );
    assert!(
        error("rect \"box\" x=0 y=0 width=1 height=1 paint=\"ink\"\nrect attach=\"box.mark\" x=0 y=0 width=1 height=1 paint=\"red\"")
            .contains("is not a placed scene"),
    );
}
