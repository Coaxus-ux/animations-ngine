use kdl::{KdlEntry, KdlNode};
use px_core::Paint;
use px_engine::{Bounds, Geometry, Param, Segment, Shape};

use crate::diagnostic::Diagnostics;
use crate::palette::NamedPalette;
use crate::props::Props;
use crate::units::Axis;
use crate::values;

pub(crate) const PROPERTIES: usize = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ShapeKind {
    Rect,
    Ellipse,
    Line,
}

const KINDS: [(&str, ShapeKind); 3] = [
    ("rect", ShapeKind::Rect),
    ("ellipse", ShapeKind::Ellipse),
    ("line", ShapeKind::Line),
];

impl ShapeKind {
    pub(crate) const fn properties(self) -> [(&'static str, Axis); PROPERTIES] {
        match self {
            Self::Rect | Self::Ellipse => [
                ("x", Axis::Horizontal),
                ("y", Axis::Vertical),
                ("width", Axis::Horizontal),
                ("height", Axis::Vertical),
            ],
            Self::Line => [
                ("x1", Axis::Horizontal),
                ("y1", Axis::Vertical),
                ("x2", Axis::Horizontal),
                ("y2", Axis::Vertical),
            ],
        }
    }

    pub(crate) fn property_index(self, name: &str) -> Option<usize> {
        self.properties()
            .iter()
            .position(|(property, _)| *property == name)
    }
}

pub(crate) struct Draft<'a> {
    pub(crate) id: Option<&'a str>,
    pub(crate) kind: ShapeKind,
    pub(crate) paint: Option<Paint>,
    pub(crate) entries: [Option<&'a KdlEntry>; PROPERTIES],
}

pub(crate) fn draft<'a>(
    node: &'a KdlNode,
    palette: &NamedPalette,
    diags: &mut Diagnostics,
) -> Option<Draft<'a>> {
    let mut props = Props::new(node);
    let Some(kind) = KINDS
        .iter()
        .find(|(name, _)| *name == props.node_name())
        .map(|(_, kind)| *kind)
    else {
        let names: Vec<&str> = KINDS.iter().map(|(name, _)| *name).collect();
        diags.error(
            props.offset(),
            format!(
                "unknown shape `{}` (expected one of: {})",
                props.node_name(),
                names.join(", ")
            ),
        );
        return None;
    };
    let id = props
        .args()
        .next()
        .and_then(|entry| values::string(entry, diags));
    let entries = kind
        .properties()
        .map(|(name, _)| props.require(name, diags));
    let paint = props
        .require("paint", diags)
        .and_then(|entry| values::string(entry, diags).map(|text| (text, entry.span().offset())))
        .and_then(|(text, offset)| palette.paint(text, offset, diags));
    props.finish(diags);
    Some(Draft {
        id,
        kind,
        paint,
        entries,
    })
}

pub(crate) fn build(kind: ShapeKind, paint: Paint, params: [Param; PROPERTIES]) -> Shape {
    let [a, b, c, d] = params;
    let geometry = match kind {
        ShapeKind::Rect => Geometry::Rect(Bounds {
            x: a,
            y: b,
            width: c,
            height: d,
        }),
        ShapeKind::Ellipse => Geometry::Ellipse(Bounds {
            x: a,
            y: b,
            width: c,
            height: d,
        }),
        ShapeKind::Line => Geometry::Line(Segment {
            x1: a,
            y1: b,
            x2: c,
            y2: d,
        }),
    };
    Shape { geometry, paint }
}
