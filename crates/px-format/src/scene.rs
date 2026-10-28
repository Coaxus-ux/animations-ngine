use std::collections::BTreeMap;

use kdl::{KdlDocument, KdlNode};
use px_core::{FrameRate, MAX_CANVAS_SIDE, MAX_FPS, Size};
use px_engine::{Environment, Looping, SceneSpec, Shape, Timing};

use crate::assemble::assemble;
use crate::compiler::Compiler;
use crate::diagnostic::Diagnostics;
use crate::palette::NamedPalette;
use crate::params::SceneParams;
use crate::props::Props;
use crate::units::Measure;
use crate::{objects, suggest, values};

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
            let hint = suggest::hint(text, palettes.keys().map(String::as_str));
            diags.error(
                entry.span().offset(),
                format!("unknown palette \"{text}\".{hint}"),
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
    let world =
        palette
            .zip(size)
            .zip(seed)
            .zip(timing)
            .and_then(|(((palette, size), seed), timing)| {
                let context = Context {
                    palette,
                    measure: Measure {
                        canvas: size,
                        fps: timing.fps,
                    },
                    seed,
                    overrides,
                };
                world(node, &context, diags)
            });
    let (shapes, environment) = world?;
    Some(SceneSpec {
        name: name?.to_owned(),
        size: size?,
        timing: timing?,
        seed: seed?,
        palette: palette?.palette.clone(),
        background: background?,
        shapes,
        environment,
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

fn world(
    node: &KdlNode,
    context: &Context<'_>,
    diags: &mut Diagnostics,
) -> Option<(Vec<Shape>, Environment)> {
    let mut params = SceneParams::default();
    let children = node.children().map(KdlDocument::nodes).unwrap_or_default();
    for param in children
        .iter()
        .filter(|child| child.name().value() == "param")
    {
        params.add(param, diags);
    }
    params.apply_overrides(context.overrides, diags);
    let tree = objects::collect(node, context.palette, diags);
    let params =
        Compiler::new(&tree.drafts, &params, context.measure, context.seed, diags).compile_all();
    if diags.has_errors() {
        return None;
    }
    Some(assemble(tree, params?))
}
