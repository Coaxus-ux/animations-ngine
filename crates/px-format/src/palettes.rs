use std::collections::{BTreeMap, BTreeSet};

use kdl::KdlNode;

use crate::diagnostic::Diagnostics;
use crate::document::scene_name;
use crate::palette::{self, Includes, NamedPalette};
use crate::suggest;

pub(crate) fn build(
    nodes: &[(usize, &KdlNode)],
    diags: &mut Diagnostics,
) -> BTreeMap<String, NamedPalette> {
    let mut building = Building {
        nodes: BTreeMap::new(),
        built: BTreeMap::new(),
        failed: BTreeSet::new(),
        open: Vec::new(),
    };
    for &(file, node) in nodes {
        let name = scene_name(node);
        if building.nodes.contains_key(name) {
            let previous = diags.in_file(file);
            diags.error(
                node.span().offset(),
                format!("palette \"{name}\" is defined twice"),
            );
            diags.in_file(previous);
        } else {
            building.nodes.insert(name, (file, node));
        }
    }
    let names: Vec<&str> = building.nodes.keys().copied().collect();
    for name in names {
        building.ensure(name, None, diags);
    }
    building.built
}

struct Building<'n> {
    nodes: BTreeMap<&'n str, (usize, &'n KdlNode)>,
    built: BTreeMap<String, NamedPalette>,
    failed: BTreeSet<String>,
    open: Vec<&'n str>,
}

impl Building<'_> {
    fn ensure(&mut self, name: &str, offset: Option<usize>, diags: &mut Diagnostics) {
        if self.built.contains_key(name) || self.failed.contains(name) {
            return;
        }
        let Some((&key, &(file, node))) = self.nodes.get_key_value(name) else {
            let hint = suggest::hint(name, self.nodes.keys().copied());
            diags.error(
                offset.unwrap_or_default(),
                format!("unknown palette \"{name}\".{hint}"),
            );
            return;
        };
        if self.open.contains(&key) {
            let chain = self.open.join(" → ");
            diags.error(
                offset.unwrap_or_default(),
                format!("palettes include each other: {chain} → {name}"),
            );
            return;
        }
        self.open.push(key);
        let previous = diags.in_file(file);
        let parsed = palette::parse(node, self, diags);
        diags.in_file(previous);
        self.open.pop();
        match parsed {
            Some((_, palette)) => {
                self.built.insert(key.to_owned(), palette);
            }
            None => {
                self.failed.insert(key.to_owned());
            }
        }
    }
}

impl Includes for Building<'_> {
    fn included(
        &mut self,
        name: &str,
        offset: usize,
        diags: &mut Diagnostics,
    ) -> Option<NamedPalette> {
        self.ensure(name, Some(offset), diags);
        self.built.get(name).cloned()
    }
}
