#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use std::path::Path;

use px_director::{Edge, TransitionKind};
use px_format::{LoadOptions, MemorySources, Work, load, load_work};

fn sources(sequence: &str) -> MemorySources {
    let red = r##"palette "red" { ramp "sky" "#200000" "#400000"; ramp "ink" "#ff0000" }
scene "dawn" palette="red" width=8 height=4 fps=4 duration="2s" loop=#true background="sky" {
    param mark "ink"
    timeline { marker "late" at="1.5s" }
    rect x="=t * 4" y=0 width=1 height=1 paint="$mark"
}"##;
    let blue = r##"palette "blue" { ramp "sea" "#000020" "#000040"; ramp "ink" "#0000ff" }
scene "dusk" palette="blue" width=8 height=4 fps=4 duration="1s" background="sea" {}
scene "small" palette="blue" width=4 height=4 fps=4 duration="1s" background="sea" {}"##;
    MemorySources::default()
        .with(
            "film/main.kdl",
            format!("use \"parts/dawn.kdl\"\nuse \"parts/dusk.kdl\"\n{sequence}"),
        )
        .with("film/parts/dawn.kdl", red)
        .with("film/parts/dusk.kdl", blue)
}

fn work(sequence: &str) -> Result<Work, Vec<String>> {
    let mut sources = sources(sequence);
    load_work(
        Path::new("film/main.kdl"),
        &mut sources,
        LoadOptions::default(),
    )
    .map_err(|error| {
        error
            .diagnostics
            .iter()
            .map(|d| d.display(&error.files))
            .collect()
    })
}

fn errors(sequence: &str) -> String {
    work(sequence).unwrap_err().join("\n")
}

#[test]
fn shots_of_scenes_in_other_palettes_make_one_sequence() {
    let loaded = match work(
        "sequence \"film\" {\n    shot \"dawn\" duration=\"1s\" from=\"late\" mark=\"sky\"\n    transition \"wipe\" duration=\"2f\" from=\"right\"\n    shot \"dusk\" duration=\"1s\"\n    shot \"dawn\" duration=\"4f\"\n}",
    )
    .unwrap()
    {
        Work::Sequence(loaded) => Some(loaded),
        Work::Scene(_) => None,
    }
    .unwrap();
    let sequence = loaded.sequence;
    assert_eq!(sequence.shots().len(), 3);
    assert_eq!(
        sequence.shots()[0].from.index(),
        6,
        "`from` names the scene's marker"
    );
    assert_eq!(
        sequence.transitions()[0].kind,
        TransitionKind::Wipe(Edge::Right)
    );
    assert_eq!(
        sequence.transitions()[1].kind,
        TransitionKind::Cut,
        "shots side by side are cut"
    );
    assert_eq!(sequence.duration().get(), 4 + 4 + 4 - 2);
    let first = &sequence.shots()[0].scene;
    assert_eq!(
        first.shapes()[0].paint,
        first.background(),
        "the shot set the scene's param"
    );
}

#[test]
fn loading_a_sequence_as_a_scene_explains_why_it_fails() {
    let mut sources = sources("sequence \"film\" { shot \"dawn\" duration=\"1s\" }");
    let error = load(
        Path::new("film/main.kdl"),
        &mut sources,
        LoadOptions::default(),
    )
    .unwrap_err();
    assert!(error.diagnostics[0].message.contains("is a sequence"));
}

#[test]
fn sequence_mistakes_are_explained() {
    assert!(
        errors("sequence \"f\" { shot \"dwan\" duration=\"1s\" }").contains("Did you mean `dawn`?")
    );
    assert!(errors("sequence \"f\" { transition \"dither\" duration=\"1s\"; shot \"dawn\" duration=\"1s\" }").contains("starts with a `shot`"));
    assert!(errors("sequence \"f\" { shot \"dawn\" duration=\"1s\"; transition \"wip\" duration=\"1s\"; shot \"dusk\" duration=\"1s\" }").contains("Did you mean `wipe`?"));
    assert!(
        errors("sequence \"f\" { shot \"dawn\" duration=\"1s\"; shot \"small\" duration=\"1s\" }")
            .contains("another size")
    );
    assert!(errors("sequence \"f\" { shot \"dawn\" duration=\"1s\"; transition \"black\" duration=\"1s\"; shot \"dusk\" duration=\"1s\" }").contains("as long as the shot"));
    assert!(
        errors("sequence \"f\" { shot \"dawn\" duration=\"1s\" from=\"lat\" }")
            .contains("Did you mean `late`?")
    );
    assert!(
        errors("sequence \"f\" { shot \"dawn\" duration=\"1s\" colour=\"sky\" }")
            .contains("colour")
    );
}
