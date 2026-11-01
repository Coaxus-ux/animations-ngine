use std::fmt;

use kdl::{KdlDocument, KdlNode};

/// Where a node sits in a document: from the top level down, each step names a node and,
/// among its siblings with the same name and id, which one. Stable across edits elsewhere.
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodePath(Vec<Step>);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Step {
    pub name: String,
    /// The node's first argument, when it is a string (`layer "sky"`).
    pub id: Option<String>,
    pub nth: usize,
}

impl NodePath {
    #[must_use]
    pub fn root() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn steps(&self) -> &[Step] {
        &self.0
    }

    /// The path of the `nth` child called `name` with `id`.
    #[must_use]
    pub fn child(&self, name: &str, id: Option<&str>, nth: usize) -> Self {
        let mut steps = self.0.clone();
        steps.push(Step {
            name: name.to_owned(),
            id: id.map(str::to_owned),
            nth,
        });
        Self(steps)
    }

    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        let (_, parent) = self.0.split_last()?;
        Some(Self(parent.to_vec()))
    }

    pub(crate) fn find<'d>(&self, document: &'d KdlDocument) -> Option<&'d KdlNode> {
        let mut nodes = document.nodes();
        let mut found = None;
        for step in &self.0 {
            let node = nodes
                .iter()
                .filter(|node| step.matches(node))
                .nth(step.nth)?;
            nodes = children_of(node);
            found = Some(node);
        }
        found
    }

    /// The path of `node` among `siblings`, under `self`.
    pub(crate) fn step_to(&self, siblings: &[KdlNode], node: &KdlNode) -> Self {
        let (name, id) = (node.name().value(), id_of(node));
        let nth = siblings
            .iter()
            .take_while(|other| !std::ptr::eq(*other, node))
            .filter(|other| other.name().value() == name && id_of(other) == id)
            .count();
        self.child(name, id, nth)
    }
}

impl Step {
    fn matches(&self, node: &KdlNode) -> bool {
        node.name().value() == self.name && id_of(node) == self.id.as_deref()
    }
}

/// The node's first argument when it is a string: what names it in paths and labels.
#[must_use]
pub fn id_of(node: &KdlNode) -> Option<&str> {
    node.entries()
        .first()
        .filter(|entry| entry.name().is_none())
        .and_then(|entry| entry.value().as_string())
}

/// The nodes of the block, or nothing when the node has no `{ … }`.
#[must_use]
pub fn children_of(node: &KdlNode) -> &[KdlNode] {
    node.children().map(KdlDocument::nodes).unwrap_or_default()
}

impl fmt::Display for NodePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, step) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str(" / ")?;
            }
            f.write_str(&step.name)?;
            if let Some(id) = &step.id {
                write!(f, " \"{id}\"")?;
            }
            if step.nth > 0 {
                write!(f, "[{}]", step.nth)?;
            }
        }
        Ok(())
    }
}
