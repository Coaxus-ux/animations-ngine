#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

use std::path::{Path, PathBuf};

use kdl::KdlValue;
use px_format::edit::{NodePath, Source};

const SCENE: &str = r#"// header comment
scene "main" width=8 {
    // a comment
    rect "a" x=1   y=2 // trailing
    layer "l" {
        rect x=3
    }
    /-rect x=9
}
"#;

fn scene() -> NodePath {
    NodePath::root().child("scene", Some("main"), 0)
}

fn layer() -> NodePath {
    scene().child("layer", Some("l"), 0)
}

fn kdl_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            kdl_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "kdl") {
            out.push(path);
        }
    }
}

#[test]
fn every_node_of_every_example_is_found_by_its_path() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    kdl_files(&repo.join("examples"), &mut files);
    kdl_files(&repo.join("benchmarks"), &mut files);
    assert_ne!(files.len(), 0, "no scene files found");
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        let source = Source::parse(text.clone()).unwrap();
        assert_eq!(source.text(), text);
        let document = kdl::KdlDocument::parse_v2(&text).unwrap();
        let mut stack: Vec<&kdl::KdlNode> = document.nodes().iter().collect();
        while let Some(node) = stack.pop() {
            let start = node.span().offset();
            let path = source.path_at(start).unwrap();
            assert_eq!(
                source.span(&path).unwrap().0,
                start,
                "{}: {path}",
                file.display()
            );
            stack.extend(
                node.children()
                    .map(kdl::KdlDocument::nodes)
                    .unwrap_or_default(),
            );
        }
    }
}

#[test]
fn setting_a_property_changes_only_its_token() {
    let mut source = Source::parse(SCENE).unwrap();
    let rect = scene().child("rect", Some("a"), 0);
    source
        .set_property(&rect, "y", &KdlValue::Integer(7))
        .unwrap();
    assert_eq!(source.text(), SCENE.replace("y=2", "y=7"));
    source
        .set_property(&rect, "paint", &KdlValue::String("ink.lit".into()))
        .unwrap();
    assert!(
        source
            .text()
            .contains("rect \"a\" x=1   y=7 paint=\"ink.lit\" // trailing")
    );
}

#[test]
fn replacing_inserting_and_removing_nodes_keep_the_rest() {
    let mut source = Source::parse(SCENE).unwrap();
    let inner = layer().child("rect", None, 0);
    source.replace(&inner, "ellipse x=4").unwrap();
    assert_eq!(source.text(), SCENE.replace("rect x=3", "ellipse x=4"));

    source
        .insert(&layer(), 0, "pixels \"p\" {\n    cel \"a\"\n}")
        .unwrap();
    assert!(source.text().contains(
        "    layer \"l\" {\n        pixels \"p\" {\n            cel \"a\"\n        }\n        ellipse x=4\n    }"
    ));

    source
        .remove(&layer().child("pixels", Some("p"), 0))
        .unwrap();
    assert_eq!(source.text(), SCENE.replace("rect x=3", "ellipse x=4"));

    source.insert(&scene(), 9, "rect x=5").unwrap();
    assert!(
        source
            .text()
            .contains("    }\n    rect x=5\n    /-rect x=9\n}")
    );
}

#[test]
fn a_parent_without_children_gets_a_block() {
    let mut source = Source::parse("scene \"s\"\n").unwrap();
    let scene = NodePath::root().child("scene", Some("s"), 0);
    source.insert(&scene, 0, "layer \"a\"").unwrap();
    assert_eq!(source.text(), "scene \"s\" {\n    layer \"a\"\n}\n");
    source.insert(&scene, 1, "layer \"b\"").unwrap();
    assert_eq!(
        source.text(),
        "scene \"s\" {\n    layer \"a\"\n    layer \"b\"\n}\n"
    );
}

#[test]
fn moving_a_child_reorders_whole_nodes() {
    let text = "scene \"s\" {\n    layer \"a\" {\n        rect x=1\n    }\n    layer \"b\"\n}\n";
    let mut source = Source::parse(text).unwrap();
    let scene = NodePath::root().child("scene", Some("s"), 0);
    source.move_child(&scene, 0, 1).unwrap();
    assert_eq!(
        source.text(),
        "scene \"s\" {\n    layer \"b\"\n    layer \"a\" {\n        rect x=1\n    }\n}\n"
    );
}

#[test]
fn an_edit_that_breaks_the_file_changes_nothing() {
    let mut source = Source::parse(SCENE).unwrap();
    assert!(source.replace(&layer(), "layer {").is_err());
    assert_eq!(source.text(), SCENE);
}
