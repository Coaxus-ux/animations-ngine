use std::collections::BTreeMap;

use kdl::{KdlDocument, KdlNode};
use px_core::{COORD_LIMIT, MAX_CANVAS_SIDE, MAX_FRAMES, Paint};
use px_engine::Cel;

use crate::diagnostic::Diagnostics;
use crate::names::Names;
use crate::props::Props;
use crate::suggest;
use crate::values;

mod write;

pub use write::write_pixels;

pub(crate) const EMPTY: char = '.';
const CHILDREN: [&str; 2] = ["legend", "cel"];

pub(crate) fn parse(node: &KdlNode, names: &Names<'_>, diags: &mut Diagnostics) -> Vec<Cel> {
    let children = node.children().map(KdlDocument::nodes).unwrap_or_default();
    let mut legend = Legend::default();
    for child in children
        .iter()
        .filter(|child| child.name().value() == "legend")
    {
        legend.read(child, names, diags);
    }
    let mut cels: Vec<Cel> = Vec::new();
    for child in children {
        match child.name().value() {
            "legend" => {}
            "cel" => {
                let Some(cel) = cel(child, &legend, diags) else {
                    continue;
                };
                if cels.iter().any(|other| other.frame == cel.frame) {
                    diags.error(
                        child.span().offset(),
                        format!("two cels start at frame {}", cel.frame),
                    );
                } else {
                    cels.push(cel);
                }
            }
            other => diags.error(
                child.span().offset(),
                format!(
                    "unknown node `{other}` in pixels (expected `legend` or `cel`).{}",
                    suggest::did_you_mean(other, CHILDREN)
                ),
            ),
        }
    }
    cels.sort_by_key(|cel| cel.frame);
    cels
}

#[derive(Default)]
struct Legend {
    codes: BTreeMap<String, Paint>,
    length: Option<usize>,
}

impl Legend {
    fn read(&mut self, node: &KdlNode, names: &Names<'_>, diags: &mut Diagnostics) {
        for entry in node.children().map(KdlDocument::nodes).unwrap_or_default() {
            let code = entry.name().value();
            let offset = entry.span().offset();
            let length = code.chars().count();
            let reserved = code.chars().all(|c| c == EMPTY);
            if !(1..=2).contains(&length) || reserved || code.contains(char::is_whitespace) {
                diags.error(
                    offset,
                    format!(
                        "legend codes are 1 or 2 characters other than `{EMPTY}`; found `{code}`"
                    ),
                );
                continue;
            }
            if self.length.is_some_and(|known| known != length) {
                diags.error(offset, "every legend code needs the same length");
                continue;
            }
            self.length = Some(length);
            let props = Props::new(entry);
            let paint = props
                .args()
                .next()
                .and_then(|value| values::string(value, diags))
                .and_then(|text| names.paint(text, offset, diags));
            props.finish(diags);
            if let Some(paint) = paint
                && self.codes.insert(code.to_owned(), paint).is_some()
            {
                diags.error(offset, format!("legend code `{code}` is defined twice"));
            }
        }
    }
}

fn cel(node: &KdlNode, legend: &Legend, diags: &mut Diagnostics) -> Option<Cel> {
    let mut props = Props::new(node);
    let offset = node.span().offset();
    let frame = props
        .get("frame")
        .and_then(|entry| values::bounded(entry, 0, MAX_FRAMES - 1, diags))
        .unwrap_or(0);
    let coordinate = |props: &mut Props<'_>, key, diags: &mut Diagnostics| {
        props
            .get(key)
            .and_then(|entry| values::bounded(entry, -COORD_LIMIT, COORD_LIMIT, diags))
            .unwrap_or(0)
    };
    let x = coordinate(&mut props, "x", diags);
    let y = coordinate(&mut props, "y", diags);
    let grid = props
        .args()
        .next()
        .and_then(|entry| values::string(entry, diags));
    props.finish(diags);
    let Some(grid) = grid else {
        diags.error(offset, "`cel` needs its rows: cel frame=0 \"\"\"…\"\"\"");
        return None;
    };
    let length = legend.length.unwrap_or(1);
    let rows: Vec<Vec<char>> = grid
        .split_whitespace()
        .map(|row| row.chars().collect())
        .collect();
    let columns = rows.first().map_or(0, Vec::len);
    if rows.iter().any(|row| row.len() != columns) || !columns.is_multiple_of(length) {
        diags.error(
            offset,
            format!("every row of a cel needs the same width, a multiple of {length}"),
        );
        return None;
    }
    let side = |count: usize| {
        u16::try_from(count)
            .ok()
            .filter(|side| (1..=MAX_CANVAS_SIDE).contains(side))
    };
    let (Some(width), Some(height)) = (side(columns / length), side(rows.len())) else {
        diags.error(
            offset,
            format!("a cel is 1..={MAX_CANVAS_SIDE} pixels on each side"),
        );
        return None;
    };
    let mut pixels = Vec::with_capacity(usize::from(width) * usize::from(height));
    for code in rows.iter().flat_map(|row| row.chunks(length)) {
        if code.iter().all(|c| *c == EMPTY) {
            pixels.push(None);
            continue;
        }
        let code: String = code.iter().collect();
        let Some(paint) = legend.codes.get(&code) else {
            diags.error(offset, format!("code `{code}` is not in the legend"));
            return None;
        };
        pixels.push(Some(*paint));
    }
    Some(Cel {
        frame,
        x,
        y,
        width,
        height,
        pixels,
    })
}
