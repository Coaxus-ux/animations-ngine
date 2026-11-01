#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use px_core::{Frame, Paint, hash_cells};
use px_engine::{FrameBuffer, Scene};
use px_format::{LoadOptions, load_scene};

const PALETTE: &str = r##"palette "p" { ramp "sky" "#000000" "#111111"; ramp "ink" "#ffffff" "#eeeeee"; ramp "red" "#ff0000" }"##;

fn scene(name: &str, props: &str, body: &str) -> String {
    format!(
        "scene \"{name}\" palette=\"p\" width=16 height=8 fps=4 duration=\"2s\" loop=#true background=\"sky\" {props} {{\n{body}\n}}\n"
    )
}

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

fn frame(scene: &Scene, index: u32) -> Vec<Paint> {
    let mut buffer = FrameBuffer::new(scene);
    buffer
        .render(scene, Frame::new(index))
        .unwrap()
        .cells()
        .to_vec()
}

fn hash(scene: &Scene, index: u32) -> u64 {
    let mut buffer = FrameBuffer::new(scene);
    hash_cells(buffer.render(scene, Frame::new(index)).unwrap().cells())
}

const BALL: &str = r#"rect x="=1 + t" y=2 width=3 height=2 paint="ink" texture=0.5"#;

#[test]
fn a_plain_placement_renders_like_the_inlined_shapes() {
    let placed = load(&format!(
        "{PALETTE}\n{}{}",
        scene("ball", "", BALL),
        scene("main", "", r#"use-scene "ball""#)
    ));
    let inlined = load(&format!("{PALETTE}\n{}", scene("main", "", BALL)));
    for index in 0..8 {
        assert_eq!(
            frame(&placed, index),
            frame(&inlined, index),
            "frame {index}"
        );
    }
}

#[test]
fn copies_are_numbered_and_equal_seeds_match() {
    let text = format!(
        "{PALETTE}\n{}{}",
        scene(
            "dot",
            "",
            r#"rect x=0 y=0 width=2 height=2 paint="ink" texture=0.6"#
        ),
        scene(
            "main",
            "",
            r#"use-scene "dot" count=3 x="=i * 5" y=1 seed=7"#
        )
    );
    let placed = load(&text);
    assert_eq!(placed.environment().instances.len(), 3);
    let cells = frame(&placed, 0);
    let block = |left: usize| -> Vec<Paint> {
        (1..3)
            .flat_map(|row| cells[row * 16 + left..row * 16 + left + 2].to_vec())
            .collect()
    };
    assert_eq!(block(0), block(5));
    assert_eq!(block(5), block(10));
}

#[test]
fn flip_mirrors_about_the_placement_point() {
    let text = format!(
        "{PALETTE}\n{}{}",
        scene("arrow", "", r#"rect x=0 y=0 width=3 height=1 paint="ink""#),
        scene("main", "", r#"use-scene "arrow" x=8 y=0 flip=#true"#)
    );
    let cells = frame(&load(&text), 0);
    let inked: Vec<usize> = (0..16)
        .filter(|x| cells[*x].ramp != cells[15].ramp)
        .collect();
    assert_eq!(inked, vec![5, 6, 7]);
}

#[test]
fn params_are_overridden_per_placement_and_can_name_paints() {
    let part = scene(
        "tag",
        "",
        "param color \"ink\"\nparam width 2\nrect x=0 y=0 width=\"=$width\" height=1 paint=\"$color\"",
    );
    let text = format!(
        "{PALETTE}\n{part}{}",
        scene("main", "", r#"use-scene "tag" color="red" width=4"#)
    );
    let cells = frame(&load(&text), 0);
    let red = cells[0];
    assert_eq!(&cells[..4], &[red; 4]);
    assert_ne!(cells[4], red);
    let default = format!(
        "{PALETTE}\n{part}{}",
        scene("main", "", r#"use-scene "tag""#)
    );
    assert_ne!(frame(&load(&default), 0)[0], red);
}

#[test]
fn placements_play_only_inside_their_window() {
    let text = format!(
        "{PALETTE}\n{}{}",
        scene("dot", "", BALL),
        scene("main", "", r#"use-scene "dot" start="0.5s" end="1s""#)
    );
    let empty = load(&format!("{PALETTE}\n{}", scene("main", "", "")));
    let placed = load(&text);
    assert_eq!(hash(&placed, 1), hash(&empty, 1), "before start");
    assert_ne!(hash(&placed, 2), hash(&empty, 2), "inside the window");
    assert_eq!(hash(&placed, 4), hash(&empty, 4), "at the end");
}

#[test]
fn references_reach_a_placement_by_id() {
    let text = format!(
        "{PALETTE}\n{}{}",
        scene("dot", "", r#"rect x=0 y=0 width=1 height=1 paint="ink""#),
        scene(
            "main",
            "",
            "use-scene \"dot\" id=\"hero\" x=3 y=0\nrect x=\"=@hero.x + 2\" y=0 width=1 height=1 paint=\"red\""
        )
    );
    let cells = frame(&load(&text), 0);
    assert_ne!(cells[3], cells[0]);
    assert_ne!(cells[5], cells[0]);
}

#[test]
fn placement_mistakes_are_reported() {
    let self_placing = format!("{PALETTE}\n{}", scene("main", "", r#"use-scene "main""#));
    assert!(
        errors(&self_placing)[0].contains("a scene can't place itself: main → main"),
        "{:?}",
        errors(&self_placing)
    );
    let unknown = format!(
        "{PALETTE}\n{}{}",
        scene("dot", "", ""),
        scene("main", "", r#"use-scene "dott""#)
    );
    assert!(errors(&unknown)[0].contains("Did you mean `dot`?"));
    let bad_param = format!(
        "{PALETTE}\n{}{}",
        scene("dot", "", "param size 2"),
        scene("main", "", r#"use-scene "dot" sise=3"#)
    );
    assert!(
        errors(&bad_param)[0].contains("no param `sise`"),
        "{:?}",
        errors(&bad_param)
    );
}

#[test]
fn markers_name_times_for_placements_and_keys() {
    let text = format!(
        "{PALETTE}\n{}{}",
        scene("dot", "", BALL),
        scene(
            "main",
            "",
            "timeline { marker \"in\" at=\"0.5s\"; marker \"out\" at=\"6f\" }\nuse-scene \"dot\" id=\"d\" start=\"in\" end=\"out\"\ncamera { k \"0s\" x=0; k \"in\" x=2 }"
        )
    );
    let options = LoadOptions {
        scene: Some("main"),
        ..LoadOptions::default()
    };
    let loaded = load_scene(&text, options).unwrap();
    let window = loaded.scene.environment().instances[0].window;
    assert_eq!((window.start, window.end), (0.5, 1.5));
    let timeline = loaded.timeline;
    let names: Vec<&str> = timeline.markers.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["in", "out"]);
    assert_eq!(timeline.tracks.len(), 1);
    assert_eq!(timeline.tracks[0].id.as_deref(), Some("d"));
}

#[test]
fn keys_are_reported_per_property_of_the_root_scene_only() {
    let text = format!(
        "{PALETTE}\n{}{}",
        scene(
            "dot",
            "",
            "rect x=0 y=0 width=1 height=1 paint=\"ink\" { anim x { k \"0s\" 0; k \"1s\" 2 } }"
        ),
        scene(
            "main",
            "",
            "timeline { marker \"in\" at=\"0.5s\" }\nuse-scene \"dot\"\ncamera { k \"0s\" x=0 zoom=1; k \"in\" x=2 zoom=2 }\nrect \"box\" y=1 width=2 height=2 paint=\"ink\" { anim x { k \"0s\" 0; k \"+3f\" 4 } }"
        )
    );
    let options = LoadOptions {
        scene: Some("main"),
        ..LoadOptions::default()
    };
    let keys = load_scene(&text, options).unwrap().timeline.keys;
    let summary: Vec<(&str, &str, &[f64])> = keys
        .iter()
        .map(|keyed| {
            (
                keyed.object.as_str(),
                keyed.property.as_str(),
                keyed.times.as_slice(),
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            ("camera", "x", &[0.0, 0.5][..]),
            ("camera", "zoom", &[0.0, 0.5][..]),
            ("box", "x", &[0.0, 0.75][..]),
        ]
    );
    assert_eq!(keys[0].origin, keys[1].origin);
    assert_ne!(keys[0].origin, keys[2].origin);
}

#[test]
fn placements_that_break_the_host_loop_are_warned_about() {
    let short = scene("blink", "", BALL).replace("duration=\"2s\"", "duration=\"1.5s\"");
    let text = format!(
        "{PALETTE}\n{short}{}",
        scene(
            "main",
            "",
            "use-scene \"blink\"\nuse-scene \"blink\" speed=0.75"
        )
    );
    let options = LoadOptions {
        scene: Some("main"),
        ..LoadOptions::default()
    };
    let warnings = load_scene(&text, options).unwrap().warnings;
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(
        warnings[0].message.contains("1.33 of its loops"),
        "{warnings:?}"
    );
}
