use std::collections::BTreeSet;

use kdl::{KdlEntry, KdlNode};

use crate::diagnostic::Diagnostics;

pub(crate) struct Props<'a> {
    node: &'a KdlNode,
    used: BTreeSet<&'a str>,
}

impl<'a> Props<'a> {
    pub(crate) fn new(node: &'a KdlNode) -> Self {
        Self {
            node,
            used: BTreeSet::new(),
        }
    }

    pub(crate) fn node_name(&self) -> &'a str {
        self.node.name().value()
    }

    pub(crate) fn offset(&self) -> usize {
        self.node.span().offset()
    }

    pub(crate) fn args(&self) -> impl Iterator<Item = &'a KdlEntry> {
        self.node
            .entries()
            .iter()
            .filter(|entry| entry.name().is_none())
    }

    pub(crate) fn name_arg(&self, diags: &mut Diagnostics) -> Option<&'a str> {
        let arg = self.args().next();
        let text = arg.and_then(|entry| entry.value().as_string());
        if text.is_none() {
            let offset = arg.map_or(self.offset(), |entry| entry.span().offset());
            diags.error(
                offset,
                format!(
                    "`{}` needs a name: {} \"my-name\"",
                    self.node_name(),
                    self.node_name()
                ),
            );
        }
        text
    }

    pub(crate) fn get(&mut self, key: &'a str) -> Option<&'a KdlEntry> {
        self.used.insert(key);
        self.node
            .entries()
            .iter()
            .rev()
            .find(|entry| entry.name().is_some_and(|name| name.value() == key))
    }

    pub(crate) fn require(
        &mut self,
        key: &'a str,
        diags: &mut Diagnostics,
    ) -> Option<&'a KdlEntry> {
        let entry = self.get(key);
        if entry.is_none() {
            diags.error(
                self.offset(),
                format!("`{}` is missing `{key}`", self.node_name()),
            );
        }
        entry
    }

    pub(crate) fn finish(self, diags: &mut Diagnostics) {
        for entry in self.node.entries() {
            if let Some(name) = entry
                .name()
                .filter(|name| !self.used.contains(name.value()))
            {
                diags.warning(
                    entry.span().offset(),
                    format!(
                        "unknown property `{}` on `{}` (ignored)",
                        name.value(),
                        self.node_name()
                    ),
                );
            }
        }
    }
}
