#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_cmp,
    reason = "tests fail loudly; compared values are exact"
)]

use px_engine::{Geometry, LightKind, Outline, Param, Scene, Shading};
use px_expr::Env;
use px_format::{LoadOptions, load_scene};

const PALETTE: &str = r##"palette "p" {
    ramp "sky" "#000000" "#222222" "#444444"
    ramp "ink" "#888888" "#aaaaaa" "#ffffff"
    ramp "ink-warm" "#884400" "#aa6600" "#ffaa00"
    twin "ink" warm="ink-warm"
}
"##;

fn source(body: &str) -> String {
    format!(
        "{PALETTE}scene \"s\" palette=\"p\" width=40 height=20 fps=10 duration=\"2s\" background=\"sky\" {{\n{body}\n}}\n"
    )
}

fn load(body: &str) -> Scene {
    match load_scene(&source(body), LoadOptions::default()) {
        Ok(loaded) => loaded.scene,
        Err(error) => panic!("{:?}", error.diagnostics),
    }
}

fn errors(body: &str) -> Vec<String> {
    let source = source(body);
    match load_scene(&source, LoadOptions::default()) {
        Ok(loaded) => panic!("expected errors, got a scene with {:?}", loaded.warnings),
        Err(error) => error
            .diagnostics
            .iter()
            .map(|d| d.message.clone())
            .collect(),
    }
}

fn at(param: &Param, time: f64) -> f64 {
    param
        .value(&Env {
            time,
            fps: 10.0,
            ..Env::default()
        })
        .unwrap()
}

fn x_of(scene: &Scene, shape: usize) -> &Param {
    match &scene.shapes()[shape].geometry {
        Geometry::Rect(bounds) | Geometry::Ellipse(bounds) => &bounds.x,
        other => panic!("{other:?}"),
    }
}

#[test]
fn every_kind_of_node_loads_into_the_model() {
    let body = r#"
        wind strength=2
        ambient darkness=1 occlusion=1 key-light=200
        camera { k "0s" x=0; k "2s" x=8 interp="smooth" }
        light-point x=10 y=5 color="warm" shadows=#true
        light-cone x=20 y=0 angle="80deg"
        light-rect x=0 y=10 width=5 height=2
        reflection x=0 y=15 width=40 height=5 axis=15
        layer "far" parallax=0.5 fog="40%" fog-paint="sky.hi" {
            rounded-rect x=0 y=0 width=6 height=4 radius=2 paint="ink" shading="dome"
        }
        group "window" clip=#true round=#true x=10 y=0 width=10 height=10 {
            polygon points="0,0 8,0 4,6" paint="ink" outline="selective"
            blob x=4 y=4 circles="0,0,3 3,1,2" paint="ink" emissive=#true
        }
        capsule x1=0 y1=0 x2=10 y2=0 radius=1.5 paint="ink" texture=0.5
        rope x1=0 y1=0 x2=20 y2=0 sag=3 paint="ink"
        particles x=0 y=0 width=40 height=2 count=30 vy=4 life="1s" paint="ink" fade=#true
        shade x=0 y=10 width=40 height=10 steps=-1 round=#true
    "#;
    let scene = load(body);
    let world = scene.environment();
    assert_eq!(scene.shapes().len(), 7);
    assert_eq!(world.layers.len(), 2);
    assert_eq!(world.layers[1].parallax, 0.5);
    assert_eq!(at(&world.layers[1].fog, 0.0), 0.4);
    assert_eq!(world.clips.len(), 1);
    assert_eq!(world.lights.len(), 3);
    assert!(world.lights[0].color.is_some() && world.lights[0].shadows);
    assert!(matches!(world.lights[2].kind, LightKind::Rect { .. }));
    assert_eq!(world.reflections.len(), 1);
    assert_eq!(world.ambient.key_light, 200.0);
    assert_eq!(world.wind.strength, 2.0);
    assert_eq!(at(&world.camera.x, 2.0), 8.0);
    let [room, polygon, blob, capsule, ..] = scene.shapes() else {
        panic!()
    };
    assert_eq!((room.style.layer, polygon.style.layer), (1, 0));
    assert!(matches!(room.style.shading, Shading::Dome { .. }));
    assert_eq!(
        (polygon.style.clips.as_slice(), blob.style.clips.as_slice()),
        (&[0][..], &[0][..])
    );
    assert!(matches!(polygon.style.outline, Outline::Selective(_)));
    assert!(blob.style.emissive);
    let Geometry::Line { width, .. } = &capsule.geometry else {
        panic!()
    };
    assert_eq!(
        at(width, 0.0),
        3.0,
        "a capsule is a line twice its radius wide"
    );
}

#[test]
fn keyframes_interpolate_loop_and_blend() {
    let body = r#"
        rect "a" y=0 width=1 height=1 paint="ink" {
            anim x { k "0s" 0; k "1s" 10; k "+1s" 30 }
        }
        rect "b" x=5 y=0 width=1 height=1 paint="ink" {
            anim x loop="repeat" blend="add" { k "0s" 0; k "1s" 10 }
        }
        rect "c" x=0 y=0 width=1 height=1 paint="ink" {
            anim x { k "0s" 0 interp="step"; k "1s" 10 }
            anim opacity { k "0s" "0%"; k "10f" "100%" }
        }
        rect "d" x="=@a.x" y=0 width=1 height=1 paint="ink"
    "#;
    let scene = load(body);
    assert_eq!(at(x_of(&scene, 0), 0.5), 5.0);
    assert_eq!(
        at(x_of(&scene, 0), 1.5),
        20.0,
        "`+1s` is relative to the previous key"
    );
    assert_eq!(
        at(x_of(&scene, 1), 1.5),
        10.0,
        "repeat wraps and add keeps the base 5"
    );
    assert_eq!(at(x_of(&scene, 2), 0.9), 0.0);
    assert_eq!(at(&scene.shapes()[2].style.opacity, 0.5), 0.5);
    assert_eq!(
        at(x_of(&scene, 3), 0.5),
        5.0,
        "references see the animated value"
    );
}

#[test]
fn mistakes_get_did_you_mean_hints() {
    let found = errors(
        r#"
        elipse x=0 y=0 width=1 height=1 paint="ink"
        rect x=0 y=0 width=1 height=1 paint="ink" { anim widht { k "0s" 1 } }
        rect x=0 y=0 width=1 height=1 paint="ink" { anim x { k "0s" 1 interp="lienar" } }
        light-point x=0 y=0 color="wram"
        camera { k "0s" z=1 }
        rect x=0 y=0 width=1 height=1 paint="inc"
    "#,
    );
    let expected = [
        "unknown node `elipse`. Did you mean `ellipse`?",
        "`rect` has no animatable property `widht`. Did you mean `width`?",
        "unknown interp \"lienar\". Did you mean `linear`?",
        "unknown light color \"wram\"; declare it with `twin` in the palette. Did you mean `warm`?",
        "`camera` keys cannot set `z`.",
        "unknown ramp \"inc\". Did you mean `ink`?",
    ];
    for message in expected {
        assert!(
            found.iter().any(|found| found.contains(message)),
            "{message}\n{found:#?}"
        );
    }
}

#[test]
fn a_replacing_track_satisfies_a_required_property() {
    let scene = load(r#"rect y=0 width=1 height=1 paint="ink" { anim x { k "0s" 3 } }"#);
    assert_eq!(at(x_of(&scene, 0), 1.0), 3.0);
    let found =
        errors(r#"rect y=0 width=1 height=1 paint="ink" { anim x blend="add" { k "0s" 3 } }"#);
    assert!(
        found.iter().any(|d| d.contains("`rect` is missing `x`")),
        "{found:?}"
    );
}
