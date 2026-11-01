#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use px_core::{Frame, Rgb};
use px_engine::{FrameBuffer, Scene};
use px_format::{LoadOptions, load_scene};

const BASE: &str = r##"palette "base" { ramp "sky" "#000000" "#111111"; ramp "lit" "#202020" "#404040"; ramp "lit-warm" "#402010" "#804020"; twin "lit" warm="lit-warm" }"##;
const UI: &str = r##"palette "ui" { ramp "ui-ink" "#ffffff" }"##;
const BOTH: &str = r##"palette "both" { include "base"; include "ui"; ramp "own" "#00ff00" }"##;

fn load(text: &str) -> Scene {
    let options = LoadOptions {
        scene: Some("main"),
        ..LoadOptions::default()
    };
    load_scene(text, options).unwrap().scene
}

fn errors(text: &str) -> Vec<String> {
    let options = LoadOptions {
        scene: Some("main"),
        ..LoadOptions::default()
    };
    load_scene(text, options)
        .unwrap_err()
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect()
}

fn colors(scene: &Scene) -> Vec<Rgb> {
    let mut buffer = FrameBuffer::new(scene);
    let canvas = buffer.render(scene, Frame::new(0)).unwrap();
    canvas
        .cells()
        .iter()
        .map(|paint| scene.palette().resolve(*paint).unwrap())
        .collect()
}

#[test]
fn an_included_palette_brings_its_ramps_in_order_before_the_own_ones() {
    let scene = load(&format!(
        "{BASE}\n{UI}\n{BOTH}\nscene \"main\" palette=\"both\" width=3 height=1 fps=1 duration=\"1s\" background=\"sky\" {{\n    rect x=0 y=0 width=1 height=1 paint=\"lit.1\"\n    rect x=1 y=0 width=1 height=1 paint=\"ui-ink\"\n    rect x=2 y=0 width=1 height=1 paint=\"own\"\n}}\n"
    ));
    let ramps = scene.palette().ramps();
    assert_eq!(ramps.len(), 5);
    assert_eq!(ramps[3].colors(), &[Rgb::new(255, 255, 255)]);
    assert_eq!(
        colors(&scene),
        [
            Rgb::new(0x40, 0x40, 0x40),
            Rgb::new(255, 255, 255),
            Rgb::new(0, 255, 0)
        ]
    );
}

#[test]
fn included_twins_keep_lighting_their_ramps() {
    let lit = |palette: &str| {
        load(&format!(
            "{BASE}\n{UI}\n{BOTH}\nscene \"main\" palette=\"{palette}\" width=1 height=1 fps=1 duration=\"1s\" background=\"sky\" {{\n    rect x=0 y=0 width=1 height=1 paint=\"lit.0\"\n    light-point x=0 y=0 reach=4 intensity=2 color=\"warm\"\n}}\n"
        ))
    };
    assert_eq!(colors(&lit("both")), colors(&lit("base")));
    assert_eq!(colors(&lit("both")), [Rgb::new(0x80, 0x40, 0x20)]);
}

#[test]
fn a_scene_of_an_included_palette_can_be_placed() {
    let text = format!(
        "{BASE}\n{UI}\n{BOTH}\nscene \"label\" palette=\"ui\" width=1 height=1 fps=1 duration=\"1s\" background=\"ui-ink\" {{\n    rect x=0 y=0 width=1 height=1 paint=\"ui-ink\"\n}}\nscene \"main\" palette=\"both\" width=2 height=1 fps=1 duration=\"1s\" background=\"sky\" {{\n    use-scene \"label\" x=1\n}}\n"
    );
    assert_eq!(
        colors(&load(&text)),
        [Rgb::new(0, 0, 0), Rgb::new(255, 255, 255)]
    );
    let foreign = text.replace("palette=\"both\" width=2", "palette=\"base\" width=2");
    assert!(
        errors(&foreign)[0].contains("must use \"base\" or a palette it includes"),
        "{:?}",
        errors(&foreign)
    );
}

#[test]
fn include_mistakes_are_reported() {
    let scene = "scene \"main\" palette=\"a\" width=1 height=1 fps=1 duration=\"1s\" background=\"sky\" { }\n";
    let clash =
        format!("{BASE}\npalette \"a\" {{ ramp \"sky\" \"#ffffff\"; include \"base\" }}\n{scene}");
    assert!(
        errors(&clash)[0].contains("ramp \"sky\" of palette \"base\" is already defined here"),
        "{:?}",
        errors(&clash)
    );
    let unknown = format!("{BASE}\npalette \"a\" {{ include \"bse\" }}\n{scene}");
    assert!(
        errors(&unknown)
            .iter()
            .any(|message| message.contains("unknown palette \"bse\". Did you mean `base`?")),
        "{:?}",
        errors(&unknown)
    );
    let cycle = format!(
        "palette \"a\" {{ ramp \"sky\" \"#000000\"; include \"b\" }}\npalette \"b\" {{ ramp \"x\" \"#000000\"; include \"a\" }}\n{scene}"
    );
    assert!(
        errors(&cycle)
            .iter()
            .any(|message| message.contains("palettes include each other: a → b → a")),
        "{:?}",
        errors(&cycle)
    );
}
