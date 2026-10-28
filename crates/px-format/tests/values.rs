#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_cmp,
    reason = "tests fail loudly; compared values are exact small integers"
)]

use px_engine::{Geometry, Scene, object_seed};
use px_expr::Env;
use px_format::{LoadOptions, load_scene};

const PALETTE: &str = r##"palette "p" { ramp "sky" "#000000" "#111111"; ramp "ink" "#ffffff" }
"##;

fn source(body: &str, scene_props: &str) -> String {
    format!(
        "{PALETTE}scene \"s\" palette=\"p\" width=40 height=20 duration=\"1f\" background=\"sky\" {scene_props} {{\n{body}\n}}\n"
    )
}

fn load(body: &str) -> Scene {
    load_with(body, "", &[])
}

fn load_with(body: &str, scene_props: &str, params: &[(&str, &str)]) -> Scene {
    let params: Vec<(String, String)> = params
        .iter()
        .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
        .collect();
    let options = LoadOptions {
        params: &params,
        ..LoadOptions::default()
    };
    match load_scene(&source(body, scene_props), options) {
        Ok(loaded) => loaded.scene,
        Err(error) => panic!("{:?}", error.diagnostics),
    }
}

fn errors(body: &str, params: &[(&str, &str)]) -> Vec<String> {
    let params: Vec<(String, String)> = params
        .iter()
        .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
        .collect();
    let options = LoadOptions {
        params: &params,
        ..LoadOptions::default()
    };
    let text = source(body, "");
    load_scene(&text, options)
        .unwrap_err()
        .diagnostics
        .iter()
        .map(|d| d.render("f", &text))
        .collect()
}

fn x_of(scene: &Scene, shape: usize) -> f64 {
    let (Geometry::Rect(bounds) | Geometry::Ellipse(bounds)) = &scene.shapes()[shape].geometry
    else {
        panic!("shape {shape} has no bounds");
    };
    let env = Env {
        seed: object_seed(scene.seed(), shape),
        ..Env::default()
    };
    bounds.x.value(&env).unwrap()
}

fn rect(x: &str) -> String {
    format!("rect x={x} y=0 width=4 height=2 paint=\"ink\"")
}

#[test]
fn length_units_convert_against_the_canvas() {
    let scene = load(&[rect("\"50%\""), rect("\"7px\""), rect("12")].join("\n"));
    assert_eq!(
        [x_of(&scene, 0), x_of(&scene, 1), x_of(&scene, 2)],
        [20.0, 7.0, 12.0]
    );
    let found = errors(
        &[rect("\"3s\""), rect("\"2hu\""), rect("\"wide\"")].join("\n"),
        &[],
    );
    assert!(
        found.iter().any(|d| d.contains("`x`: `s` is not a length")),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|d| d.contains("only available inside body plans")),
        "{found:?}"
    );
    assert!(
        found.iter().any(|d| d.contains("found \"wide\"")),
        "{found:?}"
    );
}

#[test]
fn ranges_pick_one_stable_value_per_object() {
    let scene = load(&[rect("\"2..5\""), rect("\"10%..50%\"")].join("\n"));
    let integer = x_of(&scene, 0);
    assert!([2.0, 3.0, 4.0, 5.0].contains(&integer), "{integer}");
    assert!((4.0..=20.0).contains(&x_of(&scene, 1)));
    assert_eq!(
        x_of(&load(&rect("\"2..5\"")), 0),
        integer,
        "reloading keeps the value"
    );
    let picks: Vec<f64> = (0..40)
        .map(|seed| {
            x_of(
                &load_with(&rect("\"0..1000\""), &format!("seed={seed}"), &[]),
                0,
            )
        })
        .collect();
    assert!(
        picks.windows(2).any(|pair| pair[0] != pair[1]),
        "the scene seed changes picks"
    );
    let found = errors(&rect("\"2..5%\""), &[]);
    assert!(found.iter().any(|d| d.contains("same unit")), "{found:?}");
}

#[test]
fn params_feed_expressions_and_accept_overrides() {
    let body = format!(
        "param wind 0.4 min=0 max=1 label=\"Wind\"\nparam storm #true\n{}\n{}",
        rect("\"=$wind * 10\""),
        rect("\"=$storm\"")
    );
    assert_eq!(x_of(&load(&body), 0), 4.0);
    assert_eq!(x_of(&load(&body), 1), 1.0);
    assert_eq!(x_of(&load_with(&body, "", &[("wind", "0.8")]), 0), 8.0);
    let found = errors(&body, &[("wind", "3"), ("gust", "1")]);
    assert!(
        found
            .iter()
            .any(|d| d.contains("--param wind: 3 is outside 0..=1")),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|d| d.contains("--param gust: unknown param")),
        "{found:?}"
    );
}

#[test]
fn param_errors_name_the_problem() {
    let found = errors(
        &format!(
            "param mood \"calm\" options=\"calm wild\"\n{}\n{}\n{}",
            rect("\"=$mood\""),
            rect("\"=$gust\""),
            rect("\"=$moood\"")
        ),
        &[],
    );
    assert!(
        found.iter().any(|d| d.contains("param `$mood` is text")),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|d| d.contains("unknown param `$gust`. Known: mood.")),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|d| d.contains("unknown param `$moood`. Did you mean `mood`?")),
        "{found:?}"
    );
    let found = errors("param speed 9 max=5\nparam speed 1", &[]);
    assert!(
        found.iter().any(|d| d.contains("9 is outside")),
        "{found:?}"
    );
}

#[test]
fn references_read_other_properties_in_any_order() {
    let body = [
        "rect \"follower\" x=\"=@leader.x + 2\" y=0 width=\"=@self.height * 3\" height=2 paint=\"ink\"",
        "ellipse \"leader\" x=\"=10 + t\" y=0 width=4 height=4 paint=\"ink\"",
    ]
    .join("\n");
    let scene = load(&body);
    assert_eq!(x_of(&scene, 0), 12.0);
    let Geometry::Rect(bounds) = &scene.shapes()[0].geometry else {
        panic!()
    };
    assert_eq!(bounds.width.value(&Env::default()).unwrap(), 6.0);
}

#[test]
fn references_keep_the_target_objects_seed() {
    let body = [
        rect("\"=rand() * 1000\""),
        "rect \"copy\" x=\"=@first.x\" y=0 width=1 height=1 paint=\"ink\"".to_owned(),
    ]
    .join("\n")
    .replacen("rect x=", "rect \"first\" x=", 1);
    let scene = load(&body);
    assert_eq!(x_of(&scene, 0), x_of(&scene, 1));
}

#[test]
fn reference_errors_and_cycles_are_reported_once() {
    let body = [
        "rect \"a\" x=\"=@b.x\" y=0 width=1 height=1 paint=\"ink\"",
        "rect \"b\" x=\"=@a.x + 1\" y=0 width=1 height=1 paint=\"ink\"",
        "rect \"c\" x=\"=@nobody.x\" y=\"=@a.depth\" width=1 height=1 paint=\"ink\"",
    ]
    .join("\n");
    let found = errors(&body, &[]);
    let cycles: Vec<&String> = found
        .iter()
        .filter(|d| d.contains("reference cycle"))
        .collect();
    assert_eq!(cycles.len(), 1, "{found:?}");
    assert!(cycles[0].contains("a.x → b.x → a.x"), "{found:?}");
    assert!(
        found
            .iter()
            .any(|d| d.contains("no object with id \"nobody\". Known: a, b, c.")),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|d| d.contains("`a` has no property `depth`. Known: x, y, width, height, opacity")),
        "{found:?}"
    );
}

#[test]
fn duplicate_ids_are_errors() {
    let body = "rect \"twin\" x=0 y=0 width=1 height=1 paint=\"ink\"\nrect \"twin\" x=1 y=0 width=1 height=1 paint=\"ink\"";
    let found = errors(body, &[]);
    assert!(
        found
            .iter()
            .any(|d| d.contains("id \"twin\" is used twice")),
        "{found:?}"
    );
}
