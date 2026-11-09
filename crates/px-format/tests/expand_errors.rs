#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod shorthand;

use px_format::{LoadOptions, load_scene};
use shorthand::{assert_error, file};

#[test]
fn every_misuse_reports_an_error() {
    assert_error("", "rect 1 2 3 4 5 paint=ink", "too many bare values");
    assert_error("", "rect 1 2 3 4 x=9 paint=ink", "both bare and by name");
    assert_error("", "rect $w 2 3 4 paint=ink", "write \"=$w\"");
    assert_error("", "rect 1 2 3 4 paint=ink repeat=0", "`repeat` must be");
    assert_error("", "rect 1 2 3 4 paint=ink repeat=5000", "`repeat` must be");
    assert_error(
        "",
        "rect 1 2 3 4 paint=ink repeat=\"=$n\"",
        "`repeat` must be",
    );
    assert_error("", "layer \"a\" repeat=2 { }", "cannot take `repeat`");
    assert_error("", "camera x=1 repeat=2", "cannot take `repeat`");
    assert_error("", "motif a x { rect $x 0 1 1 paint=ink; }", "top level");
    assert_error(
        "motif rect x { ellipse $x 0 1 1 paint=ink; }",
        "",
        "taken by the format",
    );
    assert_error(
        "motif a x { rect $x 0 1 1 paint=ink; }\nmotif a x { rect $x 0 1 1 paint=ink; }",
        "",
        "defined twice",
    );
    assert_error(
        "motif a k { rect $k 0 1 1 paint=ink; }",
        "",
        "cannot be a param name",
    );
    assert_error(
        "motif a repeat=1 { rect 0 0 1 1 paint=ink; }",
        "",
        "cannot be a param name",
    );
    assert_error("motif a x { layer \"l\" { } }", "", "cannot hold `layer`");
    assert_error(
        "motif a x { anim y { k \"0s\" 1; } }",
        "",
        "cannot hold `anim`",
    );
    assert_error(
        "motif a x { group { camera x=1; } }",
        "",
        "cannot hold `camera`",
    );
    assert_error("motif a", "", "has no body");
    let dot = "motif dot x { rect $x 0 1 1 paint=ink; }";
    assert_error(dot, "dot", "missing `x`");
    assert_error(dot, "dot 1 2", "too many bare values");
    assert_error(dot, "dot 1 y=2", "no param `y`");
    assert_error(dot, "dot 1 x=2", "given twice");
    assert_error(dot, "dot 1 { rect 0 0 1 1 paint=ink; }", "no children");
    assert_error(
        "motif half x { rect \"=$x/2\" 0 1 1 paint=ink; }",
        "half #true",
        "cannot go in an expression",
    );
    assert_error(
        "motif half x { rect \"=$x/2\" 0 1 1 paint=ink; }",
        "half \"60%\"",
        "cannot go in an expression",
    );
    assert_error("motif loop x { loop $x; }", "loop 1", "nest at most");
    assert_error(
        "motif big x { rect $x 0 1 1 paint=ink repeat=4096; }",
        "big 1 repeat=4096",
        "more than",
    );
}

#[test]
fn a_bare_param_name_in_an_expression_warns() {
    let text = file(
        r#"motif a x { rect "=x-2" 0 1 1 paint=ink; rect "=$x + @b.x" 0 1 1 paint=ink; }"#,
        "rect \"b\" 0 0 1 1 paint=ink\na 1",
    );
    let warnings = load_scene(&text, LoadOptions::default()).unwrap().warnings;
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].message.contains("`$x`"));
}
