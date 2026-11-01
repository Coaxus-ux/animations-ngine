use kdl::KdlNode;
use px_format::edit::{NodePath, Source};

pub(crate) use px_format::edit::{children_of, id_of};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElementKind {
    Palette,
    Ramp,
    Scene,
    Layer,
    Pixels,
    Shape,
    Use,
    UseScene,
    Other,
}

/// One row of the Elements panel: a node of the scene file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Element {
    pub path: NodePath,
    pub kind: ElementKind,
    pub label: String,
    pub depth: usize,
    /// Nodes from `use`d files can't be edited here.
    pub editable: bool,
}

/// Node names that are shapes even without a `paint=` (FORMAT appendix A).
const SHAPE_NAMES: [&str; 11] = [
    "rect",
    "ellipse",
    "rounded-rect",
    "line",
    "capsule",
    "polygon",
    "blob",
    "rope",
    "particles",
    "text",
    "shade",
];

/// Every node of `source` in file order, except the legend and cels of `pixels` nodes.
#[must_use]
pub fn elements(source: &Source) -> Vec<Element> {
    let mut rows = Vec::new();
    collect(&NodePath::root(), source.nodes(), 0, &mut rows);
    rows
}

fn collect(parent: &NodePath, siblings: &[KdlNode], depth: usize, rows: &mut Vec<Element>) {
    for node in siblings {
        let path = Source::path_of(parent, siblings, node);
        let kind = kind_of(node);
        rows.push(Element {
            path: path.clone(),
            kind,
            label: label_of(node),
            depth,
            editable: true,
        });
        if kind == ElementKind::Pixels {
            continue;
        }
        if let Some(children) = node.children() {
            collect(&path, children.nodes(), depth + 1, rows);
        }
    }
}

fn kind_of(node: &KdlNode) -> ElementKind {
    let name = node.name().value();
    match name {
        "palette" => ElementKind::Palette,
        "ramp" => ElementKind::Ramp,
        "scene" => ElementKind::Scene,
        "layer" => ElementKind::Layer,
        "pixels" => ElementKind::Pixels,
        "use" => ElementKind::Use,
        "use-scene" => ElementKind::UseScene,
        _ if SHAPE_NAMES.contains(&name) || has_property(node, "paint") => ElementKind::Shape,
        _ => ElementKind::Other,
    }
}

fn has_property(node: &KdlNode, key: &str) -> bool {
    node.entries()
        .iter()
        .any(|entry| entry.name().is_some_and(|name| name.value() == key))
}

/// `layer "sky"`, or just the name when the node has no id.
pub(crate) fn label_of(node: &KdlNode) -> String {
    match id_of(node) {
        Some(id) => format!("{} {id:?}", node.name().value()),
        None => node.name().value().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests fail loudly")]

    use super::*;

    fn room() -> Source {
        Source::parse(include_str!("../../../../examples/pixel-room.kdl")).unwrap()
    }

    #[test]
    fn the_element_tree_lists_palette_ramps_scene_layers_shapes_and_use_in_file_order() {
        let source = Source::parse(format!("use \"other.kdl\"\n{}", room().text())).unwrap();
        let rows = elements(&source);
        let summary: Vec<(ElementKind, &str, usize)> = rows
            .iter()
            .map(|row| (row.kind, row.label.as_str(), row.depth))
            .collect();
        assert_eq!(summary[0], (ElementKind::Use, "use \"other.kdl\"", 0));
        assert_eq!(summary[1], (ElementKind::Palette, "palette \"room\"", 0));
        assert_eq!(summary[2], (ElementKind::Ramp, "ramp \"wall\"", 1));
        assert_eq!(summary[8], (ElementKind::Scene, "scene \"pixel-room\"", 0));
        assert_eq!(summary[9], (ElementKind::Other, "ambient", 1));
        assert_eq!(summary[10], (ElementKind::Layer, "layer \"room\"", 1));
        assert_eq!(summary[11], (ElementKind::Shape, "rect", 2));
        assert_eq!(summary[13], (ElementKind::Other, "light-point", 2));
        assert_eq!(summary[16], (ElementKind::Layer, "layer \"cat\"", 1));
        assert_eq!(summary[17], (ElementKind::Pixels, "pixels \"cat-art\"", 2));
        assert_eq!(summary[18], (ElementKind::Layer, "layer \"layer-1\"", 1));
        assert!(rows.iter().all(|row| row.editable));
        assert!(!rows.iter().any(|row| row.label.starts_with("legend")));
        assert!(!rows.iter().any(|row| row.label.starts_with("cel")));
    }

    #[test]
    fn every_element_path_resolves_to_its_node() {
        let source = room();
        let rows = elements(&source);
        assert!(rows.len() > 15);
        for row in &rows {
            let node = source
                .node(&row.path)
                .unwrap_or_else(|| panic!("{}", row.path));
            assert_eq!(label_of(node), row.label);
        }
        let second_rect = rows
            .iter()
            .filter(|row| row.label == "rect")
            .nth(1)
            .unwrap();
        assert_eq!(second_rect.path.steps().last().unwrap().nth, 1);
    }
}
