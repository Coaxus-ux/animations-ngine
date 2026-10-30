use kdl::{KdlDocument, KdlEntry, KdlNode};
use px_core::{Frame, FrameCount, FrameRate};
use px_director::{Edge, Sequence, Shot, Transition, TransitionKind};
use px_engine::Scene;

use crate::diagnostic::Diagnostics;
use crate::document::Catalog;
use crate::extras::overrides;
use crate::props::Props;
use crate::scene::{self, Overrides};
use crate::timeline::Marker;
use crate::{suggest, units, values};

const KINDS: [&str; 5] = ["cut", "dither", "black", "white", "wipe"];
const EDGES: [&str; 4] = ["left", "right", "top", "bottom"];

/// `sequence "name" { shot …; transition …; shot … }` (DIRECTOR.md §4). Shots next to
/// each other without a transition are cut together.
pub(crate) fn parse(
    node: &KdlNode,
    file: usize,
    catalog: &Catalog<'_>,
    diags: &mut Diagnostics,
) -> Option<Sequence> {
    let props = Props::new(node);
    let name = props.name_arg(diags)?;
    props.finish(diags);
    let mut shots = Vec::new();
    let mut transitions = Vec::new();
    let mut pending: Option<Transition> = None;
    for child in node.children().map(KdlDocument::nodes).unwrap_or_default() {
        let offset = child.span().offset();
        match child.name().value() {
            "shot" => {
                if !shots.is_empty() {
                    transitions.push(pending.take().unwrap_or(Transition::CUT));
                }
                shots.extend(shot(child, file, catalog, diags));
            }
            "transition" => {
                let fps = shots.last().map(|shot: &Shot| shot.scene.timing().fps);
                match (fps, pending) {
                    (Some(fps), None) => pending = transition(child, fps, diags),
                    (None, _) => diags.error(offset, "a sequence starts with a `shot`"),
                    (_, Some(_)) => diags.error(
                        offset,
                        "two transitions in a row; put a `shot` between them",
                    ),
                }
            }
            other => diags.error(
                offset,
                format!(
                    "unknown node `{other}` in sequence (expected `shot` or `transition`).{}",
                    suggest::did_you_mean(other, ["shot", "transition"])
                ),
            ),
        }
    }
    if pending.is_some() {
        diags.error(
            node.span().offset(),
            "a sequence ends with a `shot`, not a transition",
        );
    }
    if diags.has_errors() {
        return None;
    }
    Sequence::new(name.to_owned(), shots, transitions)
        .map_err(|error| diags.error(node.span().offset(), error.to_string()))
        .ok()
}

/// Compiles the shot's scene with the shot's param overrides.
fn shot(
    node: &KdlNode,
    file: usize,
    catalog: &Catalog<'_>,
    diags: &mut Diagnostics,
) -> Option<Shot> {
    let mut props = Props::new(node);
    let scene_name = props.name_arg(diags)?;
    let duration = props.require("duration", diags);
    let from = props.get("from");
    let speed = props
        .get("speed")
        .and_then(|entry| values::number(entry, diags));
    let overrides = overrides(&mut props, diags);
    props.finish(diags);
    let Some(&(scene_file, scene_node)) = catalog.scenes.get(scene_name) else {
        let hint = suggest::hint(scene_name, catalog.scenes.keys().copied());
        diags.error(
            node.span().offset(),
            format!("unknown scene \"{scene_name}\".{hint}"),
        );
        return None;
    };
    let previous = diags.in_file(scene_file);
    let parsed = scene::parse(
        scene_node,
        catalog,
        Overrides::Instance {
            list: &overrides,
            file,
        },
        diags,
    );
    let scene = parsed.and_then(|parsed| {
        let scene = Scene::new(parsed.spec)
            .map_err(|error| diags.error(scene_node.span().offset(), error.to_string()))
            .ok()?;
        Some((scene, parsed.timeline.markers))
    });
    diags.in_file(previous);
    let (scene, markers) = scene?;
    let fps = scene.timing().fps;
    let frames =
        whole_frames(duration?, fps, &[], diags).and_then(|frames| FrameCount::new(frames).ok());
    let Some(frames) = frames else {
        diags.error(
            node.span().offset(),
            "a shot needs a `duration` of at least one frame",
        );
        return None;
    };
    let from = match from {
        Some(entry) => whole_frames(entry, fps, &markers, diags)?,
        None => 0,
    };
    Some(Shot {
        scene,
        frames,
        from: Frame::new(from),
        speed: speed.unwrap_or(1.0),
    })
}

fn transition(node: &KdlNode, fps: FrameRate, diags: &mut Diagnostics) -> Option<Transition> {
    let mut props = Props::new(node);
    let kind = props.name_arg(diags)?;
    let duration = props.get("duration");
    let edge = props.get("from").and_then(|entry| {
        let text = values::string(entry, diags)?;
        let edge = match text {
            "left" => Some(Edge::Left),
            "right" => Some(Edge::Right),
            "top" => Some(Edge::Top),
            "bottom" => Some(Edge::Bottom),
            _ => None,
        };
        if edge.is_none() {
            let hint = suggest::did_you_mean(text, EDGES);
            diags.error(
                entry.span().offset(),
                format!("a wipe comes in from left, right, top or bottom.{hint}"),
            );
        }
        edge
    });
    props.finish(diags);
    let kind = match kind {
        "cut" => TransitionKind::Cut,
        "dither" => TransitionKind::Dither,
        "black" => TransitionKind::Black,
        "white" => TransitionKind::White,
        "wipe" => TransitionKind::Wipe(edge.unwrap_or(Edge::Left)),
        other => {
            let hint = suggest::did_you_mean(other, KINDS);
            diags.error(
                node.span().offset(),
                format!("unknown transition \"{other}\".{hint}"),
            );
            return None;
        }
    };
    let frames = match duration {
        Some(entry) => whole_frames(entry, fps, &[], diags)?,
        None if kind == TransitionKind::Cut => 0,
        None => {
            diags.error(
                node.span().offset(),
                "a transition needs a `duration`, like duration=\"1s\"",
            );
            return None;
        }
    };
    Some(Transition { kind, frames })
}

/// A time value in whole frames; it can also name one of `markers`.
fn whole_frames(
    entry: &KdlEntry,
    fps: FrameRate,
    markers: &[Marker],
    diags: &mut Diagnostics,
) -> Option<u32> {
    let text = values::string(entry, diags)?;
    let seconds = markers
        .iter()
        .find(|marker| marker.name == text.trim())
        .map(|marker| marker.at * f64::from(fps.get()));
    let frames = seconds
        .or_else(|| units::literal(text).and_then(|literal| literal.frames(fps)))
        .filter(|frames| frames.is_finite() && *frames >= 0.0);
    let Some(frames) = frames else {
        let names = markers.iter().map(|marker| marker.name.as_str());
        let hint = suggest::did_you_mean(text, names);
        diags.error(
            entry.span().offset(),
            format!("invalid time \"{text}\": use a unit like \"2s\" or \"24f\", or a marker of the scene.{hint}"),
        );
        return None;
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "non-negative, finite and clamped to u32"
    )]
    let whole = frames.round().min(f64::from(u32::MAX)) as u32;
    Some(whole)
}
