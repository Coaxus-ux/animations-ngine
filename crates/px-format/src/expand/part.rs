use kdl::{KdlDocument, KdlEntry, KdlNode, KdlValue};

pub(super) const PLACED: [&str; 2] = ["x", "y"];
pub(super) const PLACEMENT: [&str; 7] = ["x", "y", "scale", "flip", "id", "attach", "seed"];
const PARAM: &str = "param";
const SHARED: [&str; 2] = [PARAM, "timeline"];
const MARK: &str = "part";
const ANCHOR: &str = "anchor";
const ANCHOR_KIND: &str = "group";

pub(crate) fn is_part(scene: &KdlNode) -> bool {
    scene.ty().is_some_and(|ty| ty.value() == MARK)
}

pub(super) struct Host {
    header: Vec<KdlEntry>,
    shared: Vec<KdlNode>,
    params: Vec<String>,
}

impl Host {
    pub(super) fn of(scene: &KdlNode) -> Self {
        let children = scene.children().map(KdlDocument::nodes).unwrap_or_default();
        let shared: Vec<KdlNode> = children
            .iter()
            .filter(|child| SHARED.contains(&child.name().value()))
            .cloned()
            .collect();
        Self {
            header: scene
                .entries()
                .iter()
                .filter(|entry| entry.name().is_some())
                .cloned()
                .collect(),
            params: shared
                .iter()
                .filter(|node| node.name().value() == PARAM)
                .filter_map(|param| param.entries().first()?.value().as_string())
                .map(str::to_owned)
                .collect(),
            shared,
        }
    }

    pub(super) fn scene(&self, name: &str, body: Vec<KdlNode>, call: &KdlNode) -> KdlNode {
        let mut scene = KdlNode::new("scene");
        scene.set_ty(MARK);
        scene.set_span(call.span());
        scene.entries_mut().push(spanned(name.into(), call));
        scene.entries_mut().extend(self.header.iter().cloned());
        let mut children = KdlDocument::new();
        children.nodes_mut().extend(self.shared.iter().cloned());
        children.nodes_mut().extend(body);
        scene.set_children(children);
        scene
    }

    pub(super) fn placement(
        &self,
        name: &str,
        placement: Vec<KdlEntry>,
        call: &KdlNode,
    ) -> KdlNode {
        let mut node = KdlNode::new("use-scene");
        node.set_span(call.span());
        node.entries_mut().push(spanned(name.into(), call));
        node.entries_mut().extend(placement);
        for param in &self.params {
            let mut forward = spanned(format!("${param}").into(), call);
            forward.set_name(Some(param.as_str()));
            node.entries_mut().push(forward);
        }
        node
    }
}

pub(super) fn anchor(node: &mut KdlNode) -> Result<(), String> {
    if node.name().value() != ANCHOR {
        return Ok(());
    }
    let named = node
        .entries()
        .first()
        .filter(|entry| entry.name().is_none())
        .and_then(|entry| entry.value().as_string())
        .is_some_and(|text| !text.starts_with('=') && !text.starts_with('$'));
    if !named || node.children().is_some() {
        return Err("`anchor` is a named point: anchor top 0 -60".to_owned());
    }
    node.set_name(ANCHOR_KIND);
    Ok(())
}

fn spanned(value: KdlValue, call: &KdlNode) -> KdlEntry {
    let mut entry = KdlEntry::new(value);
    entry.set_span(call.span());
    entry
}
