use kdl::{KdlDocument, KdlNode};
use px_core::{FrameRate, MAX_CANVAS_SIDE, MAX_FPS, Paint, Size};
use px_engine::{Looping, SceneSpec, Timing};

use crate::assemble::{Output, Scope, assemble};
use crate::compiler::Compiler;
use crate::diagnostic::Diagnostics;
use crate::document::Catalog;
use crate::extras::Override;
use crate::instances;
use crate::objects;
use crate::paints::Paints;
use crate::palette::NamedPalette;
use crate::params::SceneParams;
use crate::props::Props;
use crate::timeline::{Markers, Timeline};
use crate::units::Measure;
use crate::{suggest, values};

const DEFAULT_FPS: u16 = 12;

/// A scene's own properties, read before its contents.
pub(crate) struct Header<'c> {
    pub(crate) name: Option<&'c str>,
    pub(crate) palette_name: Option<&'c str>,
    pub(crate) palette: Option<&'c NamedPalette>,
    pub(crate) size: Option<Size>,
    pub(crate) timing: Option<Timing>,
    pub(crate) background: Option<Paint>,
    pub(crate) seed: Option<u64>,
}

/// What `%` and frame units are measured against: a scene's canvas and frame rate.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Canvas {
    pub(crate) size: Size,
    pub(crate) fps: FrameRate,
}

impl Header<'_> {
    pub(crate) fn canvas(&self) -> Option<Canvas> {
        Some(Canvas {
            size: self.size?,
            fps: self.timing?.fps,
        })
    }
}

pub(crate) fn header<'c>(
    node: &'c KdlNode,
    catalog: &'c Catalog<'_>,
    diags: &mut Diagnostics,
) -> Header<'c> {
    let mut props = Props::new(node);
    let name = props.name_arg(diags);
    let palette_name = props
        .require("palette", diags)
        .and_then(|entry| Some((entry, values::string(entry, diags)?)));
    let palette = palette_name.and_then(|(entry, text)| {
        let found = catalog.palettes.get(text);
        if found.is_none() {
            let hint = suggest::hint(text, catalog.palettes.keys().map(String::as_str));
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
    Header {
        name,
        palette_name: palette_name.map(|(_, text)| text),
        palette,
        size,
        timing,
        background,
        seed,
    }
}

pub(crate) fn parse(
    node: &KdlNode,
    catalog: &Catalog<'_>,
    overrides: &[(String, String)],
    diags: &mut Diagnostics,
) -> Option<(SceneSpec, Timeline)> {
    let header = header(node, catalog, diags);
    let (palette, palette_name) = header.palette.zip(header.palette_name)?;
    let (canvas, seed) = header.canvas().zip(header.seed)?;
    let timing = header.timing?;
    let mut builder = Builder {
        catalog,
        palette,
        palette_name,
        stack: header.name.iter().map(|name| (*name).to_owned()).collect(),
        tracking: true,
    };
    let mut out = Output::default();
    let looping = timing.looping == Looping::Loop;
    let scope = Scope {
        seed,
        loop_period: looping.then(|| seconds(timing)),
        ..Scope::default()
    };
    let markers = builder.build(
        node,
        canvas,
        &scope,
        Overrides::Cli(overrides),
        &mut out,
        diags,
    );
    if diags.has_errors() {
        return None;
    }
    let timeline = Timeline {
        duration: seconds(timing),
        looping,
        markers: markers.report(),
        tracks: out.tracks,
    };
    let spec = SceneSpec {
        name: header.name?.to_owned(),
        size: header.size?,
        timing,
        seed,
        palette: palette.palette.clone(),
        background: header.background?,
        shapes: out.shapes,
        environment: out.world,
    };
    Some((spec, timeline))
}

/// Param values given from outside a scene: on the command line, or by a `use-scene` in
/// the host's file.
#[derive(Clone, Copy)]
pub(crate) enum Overrides<'a> {
    Cli(&'a [(String, String)]),
    Instance { list: &'a [Override], file: usize },
}

/// Builds the root scene and, through `use-scene`, every scene placed in it.
pub(crate) struct Builder<'c, 'l> {
    pub(crate) catalog: &'c Catalog<'l>,
    pub(crate) palette: &'c NamedPalette,
    pub(crate) palette_name: &'c str,
    /// Names of the scenes being built, outermost first, to catch placement cycles.
    pub(crate) stack: Vec<String>,
    /// Off while building the second and later copies of a placement, whose own
    /// placements the timeline already lists once.
    pub(crate) tracking: bool,
}

impl Builder<'_, '_> {
    /// Adds the scene's objects to `out` and returns its timeline markers.
    pub(crate) fn build(
        &mut self,
        node: &KdlNode,
        canvas: Canvas,
        scope: &Scope,
        overrides: Overrides<'_>,
        out: &mut Output,
        diags: &mut Diagnostics,
    ) -> Markers {
        let markers = Markers::parse(node, canvas.fps, diags);
        let measure = Measure {
            canvas: canvas.size,
            fps: canvas.fps,
            markers: &markers,
        };
        let params = params(node, overrides, diags);
        let paints = Paints {
            palette: self.palette,
            params: &params,
        };
        let tree = objects::collect(node, &paints, diags);
        let compiled =
            Compiler::new(&tree.drafts, &params, measure, scope.seed, diags).compile_all();
        if let Some(compiled) = compiled.filter(|_| !diags.has_errors()) {
            assemble(
                tree,
                compiled,
                scope,
                out,
                diags,
                &mut |request, out, diags| {
                    instances::expand(self, &request, out, diags);
                },
            );
        }
        markers
    }
}

fn params(node: &KdlNode, overrides: Overrides<'_>, diags: &mut Diagnostics) -> SceneParams {
    let mut params = SceneParams::default();
    let children = node.children().map(KdlDocument::nodes).unwrap_or_default();
    for param in children
        .iter()
        .filter(|child| child.name().value() == "param")
    {
        params.add(param, diags);
    }
    match overrides {
        Overrides::Cli(list) => params.apply_overrides(list, diags),
        Overrides::Instance { list, file } => {
            let previous = diags.in_file(file);
            for item in list {
                if let Err(message) = params.set(&item.name, item.value.clone()) {
                    diags.error(item.offset, message);
                }
            }
            diags.in_file(previous);
        }
    }
    params
}

/// Length of a scene in seconds.
pub(crate) fn seconds(timing: Timing) -> f64 {
    timing
        .fps
        .seconds_at(px_core::Frame::new(timing.duration.get()))
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
