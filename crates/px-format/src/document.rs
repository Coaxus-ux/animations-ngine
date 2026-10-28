use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::path::Path;

use kdl::KdlNode;
use px_engine::Scene;

use crate::diagnostic::{Diagnostic, Diagnostics, LoadError};
use crate::library::Library;
use crate::palette::{self, NamedPalette};
use crate::sources::{FsSources, MemorySources, SourceFile, Sources};
use crate::timeline::Timeline;
use crate::{scene, suggest};

const MEMORY_FILE: &str = "scene.kdl";

#[derive(Debug)]
pub struct Loaded {
    pub scene: Scene,
    pub warnings: Vec<Diagnostic>,
    /// Every file the scene was built from, root first.
    pub files: Vec<SourceFile>,
    pub timeline: Timeline,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LoadOptions<'a> {
    pub scene: Option<&'a str>,
    pub params: &'a [(String, String)],
}

/// Loads a scene from a file and every file it `use`s.
///
/// # Errors
/// Returns every diagnostic of the load when any of them is an error.
pub fn load_file(path: &Path, options: LoadOptions<'_>) -> Result<Loaded, LoadError> {
    load(path, &mut FsSources, options)
}

/// Loads a scene from text that uses no other files.
///
/// # Errors
/// Returns every diagnostic of the load when any of them is an error.
pub fn load_scene(source: &str, options: LoadOptions<'_>) -> Result<Loaded, LoadError> {
    let mut sources = MemorySources::default().with(MEMORY_FILE, source);
    load(Path::new(MEMORY_FILE), &mut sources, options)
}

/// # Errors
/// Returns every diagnostic of the load when any of them is an error.
pub fn load(
    root: &Path,
    sources: &mut dyn Sources,
    options: LoadOptions<'_>,
) -> Result<Loaded, LoadError> {
    let mut diags = Diagnostics::default();
    let library = Library::load(root, sources, &mut diags);
    let catalog = Catalog::index(&library, &mut diags);
    let scene = catalog
        .select(options.scene, &mut diags)
        .and_then(|(file, node)| {
            diags.in_file(file);
            let (spec, timeline) = scene::parse(node, &catalog, options.params, &mut diags)?;
            let scene = Scene::new(spec)
                .map_err(|error| diags.error(node.span().offset(), error.to_string()))
                .ok()?;
            Some((scene, timeline))
        });
    let files = library.files;
    match (diags.finish(), scene) {
        (Ok(warnings), Some((scene, timeline))) => Ok(Loaded {
            scene,
            warnings,
            files,
            timeline,
        }),
        // Every path that yields no scene reports an error, so `finish` has already failed.
        (Ok(diagnostics) | Err(diagnostics), _) => Err(LoadError { diagnostics, files }),
    }
}

/// Palettes and scenes by name across every file of a load.
pub(crate) struct Catalog<'l> {
    pub(crate) palettes: BTreeMap<String, NamedPalette>,
    /// Each scene with the file it was written in.
    pub(crate) scenes: BTreeMap<&'l str, (usize, &'l KdlNode)>,
    root_scenes: Vec<&'l KdlNode>,
}

impl<'l> Catalog<'l> {
    fn index(library: &'l Library, diags: &mut Diagnostics) -> Self {
        let mut catalog = Self {
            palettes: BTreeMap::new(),
            scenes: BTreeMap::new(),
            root_scenes: Vec::new(),
        };
        for (file, document) in library.documents() {
            let previous = diags.in_file(file);
            for node in document.nodes() {
                catalog.add(file, node, diags);
            }
            diags.in_file(previous);
        }
        catalog
    }

    fn add(&mut self, file: usize, node: &'l KdlNode, diags: &mut Diagnostics) {
        let offset = node.span().offset();
        match node.name().value() {
            "use" => {}
            "palette" => {
                if let Some((name, palette)) = palette::parse(node, diags) {
                    match self.palettes.entry(name) {
                        Entry::Vacant(slot) => {
                            slot.insert(palette);
                        }
                        Entry::Occupied(slot) => diags.error(
                            offset,
                            format!("palette \"{}\" is defined twice", slot.key()),
                        ),
                    }
                }
            }
            "scene" => {
                let name = scene_name(node);
                if self.scenes.insert(name, (file, node)).is_some() {
                    diags.error(offset, format!("scene \"{name}\" is defined twice"));
                }
                if file == 0 {
                    self.root_scenes.push(node);
                }
            }
            other => diags.error(
                offset,
                format!(
                    "unknown top-level node `{other}` (expected `use`, `palette` or `scene`).{}",
                    suggest::did_you_mean(other, ["use", "palette", "scene"])
                ),
            ),
        }
    }

    fn select(
        &self,
        wanted: Option<&str>,
        diags: &mut Diagnostics,
    ) -> Option<(usize, &'l KdlNode)> {
        let chosen = match wanted {
            Some(wanted) => self.scenes.get(wanted).copied(),
            None => match self.root_scenes.as_slice() {
                [only] => Some((0, *only)),
                _ => None,
            },
        };
        if chosen.is_none() {
            let names: Vec<&str> = self.scenes.keys().copied().collect();
            let reason = match wanted {
                Some(wanted) => format!(
                    "no scene named \"{wanted}\".{}",
                    suggest::did_you_mean(wanted, names.iter().copied())
                ),
                None if self.root_scenes.is_empty() => "the file has no `scene`".to_owned(),
                None => "the file has several scenes; choose one".to_owned(),
            };
            let previous = diags.in_file(0);
            diags.error(0, format!("{reason} (scenes: {})", names.join(", ")));
            diags.in_file(previous);
        }
        chosen
    }
}

pub(crate) fn scene_name(node: &KdlNode) -> &str {
    node.entries()
        .first()
        .and_then(|entry| entry.value().as_string())
        .unwrap_or("")
}
