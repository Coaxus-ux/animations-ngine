use kdl::KdlNode;
use px_engine::{Bounds, Geometry, Param, Segment, Shape};

use crate::diagnostic::Diagnostics;
use crate::palette::NamedPalette;
use crate::props::Props;
use crate::values;

const SHAPES: [&str; 3] = ["rect", "ellipse", "line"];

pub(crate) fn parse(
    node: &KdlNode,
    palette: &NamedPalette,
    diags: &mut Diagnostics,
) -> Option<Shape> {
    let mut props = Props::new(node);
    let geometry = match props.node_name() {
        "rect" => bounds(&mut props, diags).map(Geometry::Rect),
        "ellipse" => bounds(&mut props, diags).map(Geometry::Ellipse),
        "line" => segment(&mut props, diags).map(Geometry::Line),
        other => {
            diags.error(
                props.offset(),
                format!(
                    "unknown shape `{other}` (expected one of: {})",
                    SHAPES.join(", ")
                ),
            );
            return None;
        }
    };
    let paint = props
        .require("paint", diags)
        .and_then(|entry| values::string(entry, diags).map(|text| (text, entry.span().offset())))
        .and_then(|(text, offset)| palette.paint(text, offset, diags));
    props.finish(diags);
    Some(Shape {
        geometry: geometry?,
        paint: paint?,
    })
}

fn param(props: &mut Props<'_>, key: &'static str, diags: &mut Diagnostics) -> Option<Param> {
    props
        .require(key, diags)
        .and_then(|entry| values::param(entry, diags))
}

fn bounds(props: &mut Props<'_>, diags: &mut Diagnostics) -> Option<Bounds> {
    let x = param(props, "x", diags);
    let y = param(props, "y", diags);
    let width = param(props, "width", diags);
    let height = param(props, "height", diags);
    Some(Bounds {
        x: x?,
        y: y?,
        width: width?,
        height: height?,
    })
}

fn segment(props: &mut Props<'_>, diags: &mut Diagnostics) -> Option<Segment> {
    let x1 = param(props, "x1", diags);
    let y1 = param(props, "y1", diags);
    let x2 = param(props, "x2", diags);
    let y2 = param(props, "y2", diags);
    Some(Segment {
        x1: x1?,
        y1: y1?,
        x2: x2?,
        y2: y2?,
    })
}
