#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod shorthand;

use px_format::{LoadOptions, load_scene};
use shorthand::{PALETTE, assert_error, assert_same, file, render};

const DOT: &str = "part dot { rect 0 0 2 2 paint=ink }";

#[test]
fn a_part_is_drawn_around_its_own_origin() {
    assert_same(DOT, "dot 5 3", "rect 5 3 2 2 paint=ink");
    assert_same(DOT, "dot", "rect 0 0 2 2 paint=ink");
}

#[test]
fn params_go_by_name_and_required_ones_follow_the_position() {
    let bar = "part bar length tone=ink { rect 0 0 $length 1 paint=$tone }";
    assert_same(bar, "bar 2 4 6", "rect 2 4 6 1 paint=ink");
    assert_same(bar, "bar 2 4 length=6 tone=red", "rect 2 4 6 1 paint=red");
}

#[test]
fn a_part_can_be_scaled_and_flipped() {
    assert_same(DOT, "dot 4 4 scale=2", "rect 4 4 4 4 paint=ink");
    assert_same(
        "part arm { rect 1 0 3 1 paint=ink }",
        "arm 10 4 flip=#true",
        "rect 6 4 3 1 paint=ink",
    );
}

#[test]
fn parts_place_parts() {
    let parts = format!("{DOT}\npart pair gap=4 {{ dot 0 0; dot $gap 1 }}");
    assert_same(
        &parts,
        "pair 3 2 gap=6",
        "rect 3 2 2 2 paint=ink\nrect 9 3 2 2 paint=ink",
    );
}

#[test]
fn a_slot_holds_the_children_of_the_call_in_the_parts_space() {
    let frame = "part frame { rect 0 0 8 1 paint=ink; group { slot } }";
    assert_same(
        frame,
        "frame 2 2 { rect 1 2 1 1 paint=red }",
        "rect 2 2 8 1 paint=ink\nrect 3 4 1 1 paint=red",
    );
    assert_same(frame, "frame 2 2", "rect 2 2 8 1 paint=ink");
}

#[test]
fn slot_children_see_the_callers_values() {
    let parts = format!("{DOT}\npart row {{ slot }}");
    assert_same(
        &parts,
        "row 1 1 repeat=2 { dot \"=$k * 4\" 0 }",
        "rect 1 1 2 2 paint=ink\nrect 5 1 2 2 paint=ink",
    );
}

#[test]
fn an_anchor_is_a_point_others_attach_to() {
    assert_same(
        "part post { rect 0 0 1 4 paint=ink; anchor top 0 -2 }",
        "post 5 8 id=p\nrect attach=p.top x=0 y=0 width=1 height=1 paint=red",
        "rect 5 8 1 4 paint=ink\nrect 5 6 1 1 paint=red",
    );
}

#[test]
fn a_part_runs_on_the_hosts_clock_and_reads_its_params() {
    assert_same(
        "part runner speed=4 { rect \"=t * $speed\" 0 1 1 paint=$tint }",
        "param tint \"red\"\nrunner 0 2 speed=6",
        "rect \"=t * 6\" 2 1 1 paint=red",
    );
}

#[test]
fn repeat_and_motifs_work_on_part_calls() {
    let parts = format!("{DOT}\nmotif trio y {{ dot \"=$k * 4\" $y repeat=3 }}");
    assert_same(
        &parts,
        "trio 6",
        "rect 0 6 2 2 paint=ink\nrect 4 6 2 2 paint=ink\nrect 8 6 2 2 paint=ink",
    );
}

#[test]
fn a_file_with_parts_still_has_one_scene_to_load() {
    let loaded = load_scene(&file(DOT, "dot 1 1 repeat=3"), LoadOptions::default()).unwrap();
    assert_eq!(loaded.scene.name(), "main");
    assert_eq!(loaded.timeline.tracks.len(), 0);
    let part = loaded.motifs.iter().find(|motif| motif.name == "dot");
    assert_eq!(part.unwrap().positional().collect::<Vec<_>>(), ["x", "y"]);
}

#[test]
fn a_plain_group_moves_its_children() {
    assert_same(
        "",
        "group g 5 3 { rect 0 0 2 2 paint=ink; group 2 0 { rect 1 1 1 1 paint=red } }",
        "rect 5 3 2 2 paint=ink\nrect 8 4 1 1 paint=red",
    );
    assert_same(
        "",
        "group g \"=t * 4\" 3 { rect 0 0 2 2 paint=ink }",
        "rect \"=t * 4\" 3 2 2 paint=ink",
    );
}

#[test]
fn a_clip_group_stays_a_window_onto_the_scene() {
    let clipped = render(&file(
        "",
        "group 5 3 4 4 clip=#true { rect 5 3 2 2 paint=ink }",
    ));
    assert_eq!(clipped, render(&file("", "rect 5 3 2 2 paint=ink")));
}

#[test]
fn bad_parts_and_calls_are_reported() {
    assert_error(DOT, "dot 1 1 { rect 0 0 1 1 paint=red }", "has no `slot`");
    assert_error(DOT, "dot 1 1 scael=2", "no param `scael`");
    assert_error(DOT, "dot 1 2 3", "too many bare values");
    assert_error("", "slot", "`slot` belongs in the body of a `part`");
    assert_error("", "part inner { rect 0 0 1 1 paint=ink }", "top level");
    assert_error("", "anchor 3 4", "`anchor` is a named point");
    assert_error(
        "part bad scale=2 { rect 0 0 1 1 paint=ink }",
        "rect 0 0 1 1 paint=ink",
        "`scale` cannot be a param name",
    );
    assert_error(
        "motif bad { slot }",
        "rect 0 0 1 1 paint=ink",
        "cannot hold `slot`",
    );
    assert_error(
        "",
        "group 2 2 { group 0 0 4 4 clip=#true { rect 0 0 1 1 paint=ink } }",
        "can't go inside a group that moves its children",
    );
}

#[test]
fn a_mistake_in_a_body_is_reported_where_it_is_written() {
    let text = format!(
        "{PALETTE}\npart odd {{ rect 0 0 1 1 paint=nope }}\n{}",
        shorthand::scene("odd 1 1")
    );
    let error = load_scene(&text, LoadOptions::default()).unwrap_err();
    let written = text.find("paint=nope").unwrap();
    assert_eq!(error.diagnostics[0].offset, written);
}
