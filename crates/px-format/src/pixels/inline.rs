use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kdl::{KdlDocument, KdlEntry, KdlNode, KdlValue};
use px_core::Rgb;

use super::EMPTY;
use super::image::Image;
use crate::diagnostic::Diagnostics;
use crate::sources::{self, SourceFile, Sources};

const CODES: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const USAGE: &str = "pixels src=\"art.png\" { legend { \"#3a2a1c\" wood.2 } }";

pub(crate) fn inline(
    documents: &mut [Option<KdlDocument>],
    files: &[SourceFile],
    sources: &mut dyn Sources,
    diags: &mut Diagnostics,
) -> Vec<PathBuf> {
    let mut images = Images {
        sources,
        assets: Vec::new(),
    };
    for (file, document) in documents.iter_mut().enumerate() {
        let (Some(document), Some(source)) = (document, files.get(file)) else {
            continue;
        };
        let previous = diags.in_file(file);
        images.block(document, &source.path, diags);
        diags.in_file(previous);
    }
    images.assets
}

struct Images<'s> {
    sources: &'s mut dyn Sources,
    assets: Vec<PathBuf>,
}

impl Images<'_> {
    fn block(&mut self, block: &mut KdlDocument, from: &Path, diags: &mut Diagnostics) {
        for node in block.nodes_mut() {
            if node.name().value() == "pixels" && node.entry("src").is_some() {
                if let Err(message) = self.replace(node, from) {
                    diags.error(node.span().offset(), message);
                }
            } else if let Some(children) = node.children_mut() {
                self.block(children, from, diags);
            }
        }
    }

    fn replace(&mut self, node: &mut KdlNode, from: &Path) -> Result<(), String> {
        let position = node
            .entries()
            .iter()
            .position(|entry| entry.name().is_some_and(|name| name.value() == "src"))
            .ok_or("`pixels` has no `src`")?;
        let entry = node.entries_mut().remove(position);
        let src = entry
            .value()
            .as_string()
            .ok_or_else(|| format!("`src` names a PNG file: {USAGE}"))?;
        let children = node.children().map(KdlDocument::nodes).unwrap_or_default();
        if children.iter().any(|child| child.name().value() == "cel") {
            return Err(
                "`pixels` takes its art from `src` or from `cel` rows, not both".to_owned(),
            );
        }
        let chosen = chosen_paints(children)?;
        let path = sources::resolve(from, src);
        if !self.assets.contains(&path) {
            self.assets.push(path.clone());
        }
        let image = self
            .sources
            .read_bytes(&path)
            .and_then(|bytes| Image::decode(&bytes))
            .map_err(|error| format!("src \"{src}\": {error}"))?;
        let art = Art::of(&image, &chosen)?;
        let block = node.ensure_children();
        block.nodes_mut().clear();
        block.nodes_mut().push(art.legend(&entry));
        block.nodes_mut().push(art.cel(&image, &entry));
        Ok(())
    }
}

fn chosen_paints(children: &[KdlNode]) -> Result<BTreeMap<String, KdlValue>, String> {
    let mut chosen = BTreeMap::new();
    let legends = children
        .iter()
        .filter(|child| child.name().value() == "legend");
    for color in legends.flat_map(|legend| {
        legend
            .children()
            .map(KdlDocument::nodes)
            .unwrap_or_default()
    }) {
        let key = color.name().value();
        let rgb = Rgb::parse_hex(key)
            .map_err(|_| format!("with `src`, legend keys are colors: {USAGE}"))?;
        let paint = color
            .entries()
            .first()
            .map(KdlEntry::value)
            .ok_or_else(|| format!("legend color \"{key}\" needs a paint: {USAGE}"))?;
        chosen.insert(rgb.to_string(), paint.clone());
    }
    Ok(chosen)
}

struct Art {
    codes: BTreeMap<String, String>,
    paints: Vec<(String, KdlValue)>,
    length: usize,
}

impl Art {
    fn of(image: &Image, chosen: &BTreeMap<String, KdlValue>) -> Result<Self, String> {
        let mut colors: Vec<String> = Vec::new();
        for rgb in image.pixels.iter().flatten() {
            let hex = rgb.to_string();
            if !colors.contains(&hex) {
                colors.push(hex);
            }
        }
        let length = if colors.len() <= CODES.len() { 1 } else { 2 };
        if colors.len() > CODES.len() * CODES.len() {
            return Err(format!(
                "the image has {} colors; at most {} fit",
                colors.len(),
                CODES.len() * CODES.len()
            ));
        }
        let mut codes = BTreeMap::new();
        let mut paints = Vec::with_capacity(colors.len());
        for (index, hex) in colors.into_iter().enumerate() {
            let code = code(index, length);
            let paint = chosen
                .get(&hex)
                .cloned()
                .unwrap_or_else(|| KdlValue::String(hex.clone()));
            paints.push((code.clone(), paint));
            codes.insert(hex, code);
        }
        Ok(Self {
            codes,
            paints,
            length,
        })
    }

    fn legend(&self, src: &KdlEntry) -> KdlNode {
        let span = src.span();
        let mut legend = KdlNode::new("legend");
        let block = legend.ensure_children();
        for (code, paint) in &self.paints {
            let mut row = KdlNode::new(code.as_str());
            let mut entry = KdlEntry::new(paint.clone());
            entry.set_span(span);
            row.entries_mut().push(entry);
            row.set_span(span);
            block.nodes_mut().push(row);
        }
        legend.set_span(span);
        legend
    }

    fn cel(&self, image: &Image, src: &KdlEntry) -> KdlNode {
        let span = src.span();
        let empty = EMPTY.to_string().repeat(self.length);
        let rows: Vec<String> = image
            .pixels
            .chunks(image.width)
            .map(|row| {
                row.iter()
                    .map(|pixel| {
                        pixel
                            .and_then(|rgb| self.codes.get(&rgb.to_string()))
                            .map_or(empty.as_str(), String::as_str)
                    })
                    .collect()
            })
            .collect();
        let mut cel = KdlNode::new("cel");
        let mut entry = KdlEntry::new(KdlValue::String(rows.join("\n")));
        entry.set_span(span);
        cel.entries_mut().push(entry);
        cel.set_span(span);
        cel
    }
}

fn code(index: usize, length: usize) -> String {
    let letter = |at: usize| char::from(CODES[at % CODES.len()]);
    if length == 1 {
        letter(index).to_string()
    } else {
        [letter(index / CODES.len()), letter(index)]
            .iter()
            .collect()
    }
}
