use kdl::KdlNode;
use px_format::edit::Source;

use super::PaletteModel;
use crate::domain::tree::{children_of, id_of, label_of};

/// Every property the loader reads as a paint (`ramp` or `ramp.tone`), as px-format's
/// scene, style, extras and objects parsers name them.
const PAINT_KEYS: [&str; 4] = ["paint", "background", "outline-paint", "fog-paint"];

/// One place in the file that names a ramp or a paint, and who that place is.
struct Use {
    paint: String,
    user: String,
}

impl PaletteModel {
    /// Who uses ramp `ramp` in `source`: the first twin link, legend code or paint
    /// property naming it, in file order.
    #[must_use]
    pub fn uses_of(source: &Source, ramp: &str) -> Option<String> {
        uses(source)
            .into_iter()
            .find(|found| {
                found.paint == ramp
                    || found
                        .paint
                        .strip_prefix(ramp)
                        .is_some_and(|rest| rest.starts_with('.'))
            })
            .map(|found| found.user)
    }

    /// Who paints with a tone of `ramp` that a ramp of `kept` tones no longer has: the
    /// tone's index and its first user. Named tones (`deep` … `hi`) fit any length.
    #[must_use]
    pub fn uses_of_tones(source: &Source, ramp: &str, kept: usize) -> Option<(usize, String)> {
        uses(source).into_iter().find_map(|found| {
            let tone = found
                .paint
                .strip_prefix(ramp)?
                .strip_prefix('.')?
                .parse::<usize>()
                .ok()
                .filter(|tone| *tone >= kept)?;
            Some((tone, found.user))
        })
    }

    /// The ramps whose tone count must equal `ramp`'s: its twins when it is a twin's base,
    /// and the base when it is a twin.
    #[must_use]
    pub fn twin_partners(source: &Source, ramp: &str) -> Vec<String> {
        let mut partners = Vec::new();
        for (base, colors) in source
            .nodes()
            .iter()
            .flat_map(children_of)
            .filter_map(twin_links)
        {
            if base == ramp {
                partners.extend(colors.iter().map(|(_, twin)| (*twin).to_owned()));
            } else if colors.iter().any(|(_, twin)| *twin == ramp) {
                partners.push(base.to_owned());
            }
        }
        partners.sort();
        partners.dedup();
        partners
    }
}

/// Every ramp and paint reference in the file, in file order.
fn uses(source: &Source) -> Vec<Use> {
    let mut found = Vec::new();
    let mut stack: Vec<&KdlNode> = source.nodes().iter().rev().collect();
    while let Some(node) = stack.pop() {
        let children = children_of(node);
        if let Some((base, colors)) = twin_links(node) {
            found.push(Use {
                paint: base.to_owned(),
                user: format!("twin {base:?}"),
            });
            found.extend(colors.into_iter().map(|(color, twin)| Use {
                paint: twin.to_owned(),
                user: format!("light color {color:?} of twin {base:?}"),
            }));
        }
        if node.name().value() == "pixels" {
            let codes = children
                .iter()
                .filter(|child| child.name().value() == "legend")
                .flat_map(children_of);
            found.extend(codes.filter_map(|code| {
                Some(Use {
                    paint: id_of(code)?.to_owned(),
                    user: format!(
                        "legend code {:?} in {}",
                        code.name().value(),
                        label_of(node)
                    ),
                })
            }));
        }
        found.extend(node.entries().iter().filter_map(|entry| {
            let key = entry.name()?.value();
            PAINT_KEYS.contains(&key).then_some(())?;
            Some(Use {
                paint: entry.value().as_string()?.to_owned(),
                user: format!("{key} of {}", label_of(node)),
            })
        }));
        stack.extend(children.iter().rev());
    }
    found
}

/// A `twin` node's base ramp and its `light-color="ramp"` entries.
fn twin_links(node: &KdlNode) -> Option<(&str, Vec<(&str, &str)>)> {
    if node.name().value() != "twin" {
        return None;
    }
    let colors = node
        .entries()
        .iter()
        .filter_map(|entry| Some((entry.name()?.value(), entry.value().as_string()?)))
        .collect();
    Some((id_of(node)?, colors))
}
