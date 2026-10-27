use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use kdl::{KdlDocument, KdlError, KdlNode};
use px_engine::Scene;

use crate::diagnostic::{Diagnostic, Diagnostics, LoadError};
use crate::palette::{self, NamedPalette};
use crate::scene;

#[derive(Debug)]
pub struct Loaded {
    pub scene: Scene,
    pub warnings: Vec<Diagnostic>,
}

pub fn load_scene(source: &str, scene_name: Option<&str>) -> Result<Loaded, LoadError> {
    let mut diags = Diagnostics::default();
    let document = KdlDocument::parse_v2(source).map_err(|error| syntax_errors(&error))?;

    let palettes = palettes(&document, &mut diags);
    let node = select_scene(&document, scene_name, &mut diags);
    let spec = node.and_then(|node| scene::parse(node, &palettes, &mut diags));
    let scene = node.zip(spec).and_then(|(node, spec)| {
        Scene::new(spec)
            .map_err(|error| diags.error(node.span().offset(), error.to_string()))
            .ok()
    });
    let warnings = diags.finish()?;
    // Every path that yields no scene reports an error, so `finish` has already failed.
    let scene = scene.ok_or_else(|| LoadError {
        diagnostics: warnings.clone(),
    })?;
    Ok(Loaded { scene, warnings })
}

fn syntax_errors(error: &KdlError) -> LoadError {
    let mut diags = Diagnostics::default();
    for diagnostic in &error.diagnostics {
        let message = diagnostic
            .message
            .clone()
            .unwrap_or_else(|| "invalid KDL syntax".to_owned());
        diags.error(diagnostic.span.offset(), message);
    }
    if !diags.has_errors() {
        diags.error(0, "invalid KDL syntax");
    }
    match diags.finish() {
        Err(error) => error,
        Ok(warnings) => LoadError {
            diagnostics: warnings,
        },
    }
}

fn palettes(document: &KdlDocument, diags: &mut Diagnostics) -> BTreeMap<String, NamedPalette> {
    let mut palettes = BTreeMap::new();
    for node in document
        .nodes()
        .iter()
        .filter(|node| node.name().value() == "palette")
    {
        if let Some((name, palette)) = palette::parse(node, diags) {
            match palettes.entry(name) {
                Entry::Vacant(slot) => {
                    slot.insert(palette);
                }
                Entry::Occupied(slot) => diags.error(
                    node.span().offset(),
                    format!("palette \"{}\" is defined twice", slot.key()),
                ),
            }
        }
    }
    palettes
}

fn select_scene<'a>(
    document: &'a KdlDocument,
    wanted: Option<&str>,
    diags: &mut Diagnostics,
) -> Option<&'a KdlNode> {
    let mut scenes = Vec::new();
    for node in document.nodes() {
        match node.name().value() {
            "palette" => {}
            "scene" => scenes.push(node),
            other => diags.error(
                node.span().offset(),
                format!("unknown top-level node `{other}` (expected `palette` or `scene`)"),
            ),
        }
    }
    let names: Vec<&str> = scenes.iter().map(|node| scene_name(node)).collect();
    let chosen = match wanted {
        Some(wanted) => scenes
            .iter()
            .copied()
            .find(|node| scene_name(node) == wanted),
        None if scenes.len() == 1 => scenes.first().copied(),
        None => None,
    };
    if chosen.is_none() {
        let reason = match wanted {
            Some(wanted) => format!("no scene named \"{wanted}\""),
            None if scenes.is_empty() => "the file has no `scene`".to_owned(),
            None => "the file has several scenes; choose one".to_owned(),
        };
        diags.error(0, format!("{reason} (scenes: {})", names.join(", ")));
    }
    chosen
}

fn scene_name(node: &KdlNode) -> &str {
    node.entries()
        .first()
        .and_then(|entry| entry.value().as_string())
        .unwrap_or("")
}
