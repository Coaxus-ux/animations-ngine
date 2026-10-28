use std::collections::BTreeMap;

use kdl::{KdlDocument, KdlNode};
use px_core::FrameRate;

use crate::diagnostic::Diagnostics;
use crate::props::Props;
use crate::units;
use crate::values;

/// Named points in a scene's time, written in its `timeline` block.
#[derive(Clone, Debug, Default)]
pub(crate) struct Markers(BTreeMap<String, f64>);

impl Markers {
    pub(crate) fn get(&self, name: &str) -> Option<f64> {
        self.0.get(name).copied()
    }

    /// Reads every `marker "name" at="2s"` inside the scene's `timeline` blocks.
    pub(crate) fn parse(scene: &KdlNode, fps: FrameRate, diags: &mut Diagnostics) -> Self {
        let mut markers = Self::default();
        let timelines = children(scene)
            .iter()
            .filter(|node| node.name().value() == "timeline");
        for node in timelines.flat_map(children) {
            if node.name().value() != "marker" {
                diags.error(
                    node.span().offset(),
                    format!(
                        "`timeline` holds `marker` nodes, found `{}`",
                        node.name().value()
                    ),
                );
                continue;
            }
            let mut props = Props::new(node);
            let name = props.name_arg(diags);
            let at = props
                .require("at", diags)
                .and_then(|entry| Some((entry, values::string(entry, diags)?)));
            props.finish(diags);
            let (Some(name), Some((entry, text))) = (name, at) else {
                continue;
            };
            match seconds(text, fps) {
                Some(at) if markers.0.insert(name.to_owned(), at).is_some() => {
                    diags.error(
                        node.span().offset(),
                        format!("marker \"{name}\" is defined twice"),
                    );
                }
                Some(_) => {}
                None => diags.error(
                    entry.span().offset(),
                    format!("marker time \"{text}\" is not a time; use \"2s\" or \"24f\""),
                ),
            }
        }
        markers
    }

    /// Every marker, earliest first.
    pub(crate) fn report(&self) -> Vec<Marker> {
        let mut markers: Vec<Marker> = self
            .0
            .iter()
            .map(|(name, at)| Marker {
                name: name.clone(),
                at: *at,
            })
            .collect();
        markers.sort_by(|a, b| a.at.total_cmp(&b.at));
        markers
    }
}

fn children(node: &KdlNode) -> &[KdlNode] {
    node.children().map(KdlDocument::nodes).unwrap_or_default()
}

fn seconds(text: &str, fps: FrameRate) -> Option<f64> {
    units::literal(text)?.seconds(fps)
}

/// What plays when in the root scene, for `pxe timeline` and the editor.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Timeline {
    pub duration: f64,
    pub looping: bool,
    pub markers: Vec<Marker>,
    pub tracks: Vec<Track>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Marker {
    pub name: String,
    pub at: f64,
}

/// One `use-scene`, with times in its host's seconds.
#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    pub scene: String,
    pub id: Option<String>,
    /// How many scenes it is nested in below the root.
    pub depth: usize,
    pub count: u32,
    pub start: f64,
    pub end: f64,
    pub offset: f64,
    pub speed: f64,
    /// Length of the placed scene.
    pub duration: f64,
    pub looping: bool,
}
