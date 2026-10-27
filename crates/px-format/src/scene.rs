use std::collections::{BTreeMap, BTreeSet};

use kdl::{KdlDocument, KdlNode};
use px_core::{FrameRate, MAX_CANVAS_SIDE, MAX_FPS, Size};
use px_engine::{Looping, MAX_SHAPES, SceneSpec, Shape, Timing};

use crate::compiler::Compiler;
use crate::diagnostic::Diagnostics;
use crate::palette::NamedPalette;
use crate::params::SceneParams;
use crate::props::Props;
use crate::shapes::Draft;
use crate::units::Measure;
use crate::{shapes, values};

const DEFAULT_FPS: u16 = 12;

pub(crate) fn parse(
    node: &KdlNode,
    palettes: &BTreeMap<String, NamedPalette>,
    overrides: &[(String, String)],
    diags: &mut Diagnostics,
) -> Option<SceneSpec> {
    let mut props = Props::new(node);
    let name = props.name_arg(diags);
    let palette = props.require("palette", diags).and_then(|entry| {
        let text = values::string(entry, diags)?;
        let found = palettes.get(text);
        if found.is_none() {
            let known: Vec<&str> = palettes.keys().map(String::as_str).collect();
            diags.error(
                entry.span().offset(),
                format!("unknown palette \"{text}\" (known: {})", known.join(", ")),
            );
        }
        found
    });
    let size = size(&mut props, diags);
    let timing = timing(&mut props, diags);
    let background =
        palette
            .zip(props.require("background", diags))
            .and_then(|(palette, entry)| {
                let text = values::string(entry, diags)?;
                palette.paint(text, entry.span().offset(), diags)
            });
    let seed = match props.get("seed") {
        Some(entry) => values::bounded(entry, 0, u64::MAX, diags),
        None => Some(0),
    };
    props.finish(diags);
    let shapes = palette
        .zip(size)
        .zip(seed)
        .and_then(|((palette, size), seed)| {
            let context = Context {
                palette,
                measure: Measure { canvas: size },
                seed,
                overrides,
            };
            shapes(node, &context, diags)
        });
    Some(SceneSpec {
        name: name?.to_owned(),
        size: size?,
        timing: timing?,
        seed: seed?,
        palette: palette?.palette.clone(),
        background: background?,
        shapes: shapes?,
    })
}

fn size(props: &mut Props<'_>, diags: &mut Diagnostics) -> Option<Size> {
    let width = props
        .require("width", diags)
        .and_then(|e| values::bounded(e, 1, MAX_CANVAS_SIDE, diags));
    let height = props
        .require("height", diags)
        .and_then(|e| values::bounded(e, 1, MAX_CANVAS_SIDE, diags));
    Size::new(width?, height?).ok()
}

fn timing(props: &mut Props<'_>, diags: &mut Diagnostics) -> Option<Timing> {
    let fps = match props.get("fps") {
        Some(entry) => values::bounded(entry, 1, MAX_FPS, diags)?,
        None => DEFAULT_FPS,
    };
    let fps = FrameRate::new(fps).ok()?;
    let duration = props
        .require("duration", diags)
        .and_then(|entry| values::duration(entry, fps, diags));
    let looping = match props.get("loop") {
        Some(entry) => values::boolean(entry, diags)?,
        None => false,
    };
    let looping = if looping {
        Looping::Loop
    } else {
        Looping::Once
    };
    Some(Timing {
        fps,
        duration: duration?,
        looping,
    })
}

struct Context<'a> {
    palette: &'a NamedPalette,
    measure: Measure,
    seed: u64,
    overrides: &'a [(String, String)],
}

fn shapes(node: &KdlNode, context: &Context<'_>, diags: &mut Diagnostics) -> Option<Vec<Shape>> {
    let children = node.children().map(KdlDocument::nodes).unwrap_or_default();
    let (param_nodes, shape_nodes): (Vec<&KdlNode>, Vec<&KdlNode>) = children
        .iter()
        .partition(|child| child.name().value() == "param");
    if shape_nodes.len() > MAX_SHAPES {
        diags.error(
            node.span().offset(),
            format!(
                "scene has {} shapes, max is {MAX_SHAPES}",
                shape_nodes.len()
            ),
        );
        return None;
    }
    let mut params = SceneParams::default();
    for param in param_nodes {
        params.add(param, diags);
    }
    params.apply_overrides(context.overrides, diags);
    let drafts: Vec<Draft<'_>> = shape_nodes
        .iter()
        .filter_map(|child| shapes::draft(child, context.palette, diags))
        .collect();
    let unique = unique_ids(&drafts, &shape_nodes, diags);
    let shapes = Compiler::new(&drafts, &params, context.measure, context.seed, diags).shapes();
    shapes.filter(|shapes| unique && shapes.len() == shape_nodes.len())
}

fn unique_ids(drafts: &[Draft<'_>], nodes: &[&KdlNode], diags: &mut Diagnostics) -> bool {
    let mut seen = BTreeSet::new();
    let mut unique = true;
    for (draft, node) in drafts.iter().zip(nodes) {
        if let Some(id) = draft.id.filter(|id| !seen.insert(*id)) {
            diags.error(
                node.span().offset(),
                format!("id \"{id}\" is used twice in this scene"),
            );
            unique = false;
        }
    }
    unique
}
