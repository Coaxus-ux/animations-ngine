use std::collections::BTreeMap;

use kdl::{KdlDocument, KdlNode};
use px_core::{FrameRate, MAX_CANVAS_SIDE, MAX_FPS, Size};
use px_engine::{Looping, MAX_SHAPES, SceneSpec, Timing};

use crate::diagnostic::Diagnostics;
use crate::palette::NamedPalette;
use crate::props::Props;
use crate::{shapes, values};

const DEFAULT_FPS: u16 = 12;

pub(crate) fn parse(
    node: &KdlNode,
    palettes: &BTreeMap<String, NamedPalette>,
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
    let shapes = palette.map(|palette| shapes(node, palette, diags));
    props.finish(diags);
    Some(SceneSpec {
        name: name?.to_owned(),
        size: size?,
        timing: timing?,
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

fn shapes(
    node: &KdlNode,
    palette: &NamedPalette,
    diags: &mut Diagnostics,
) -> Vec<px_engine::Shape> {
    let children = node.children().map(KdlDocument::nodes).unwrap_or_default();
    if children.len() > MAX_SHAPES {
        diags.error(
            node.span().offset(),
            format!("scene has {} shapes, max is {MAX_SHAPES}", children.len()),
        );
        return Vec::new();
    }
    children
        .iter()
        .filter_map(|child| shapes::parse(child, palette, diags))
        .collect()
}
