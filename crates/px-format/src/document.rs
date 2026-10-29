use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::path::Path;

use kdl::KdlNode;
use px_director::Sequence;
use px_engine::Scene;

use crate::diagnostic::{Diagnostic, Diagnostics, LoadError, Severity};
use crate::font::{self, DEFAULT_FONT, Fonts};
use crate::library::Library;
use crate::palette::{self, NamedPalette};
use crate::scene::Overrides;
use crate::sources::{FsSources, MemorySources, SourceFile, Sources};
use crate::timeline::Timeline;
use crate::{scene, sequence, suggest};

const MEMORY_FILE: &str = "scene.kdl";

#[derive(Debug)]
pub struct Loaded {
    pub scene: Scene,
    pub warnings: Vec<Diagnostic>,
    /// Every file the scene was built from, root first.
    pub files: Vec<SourceFile>,
    pub timeline: Timeline,
}

#[derive(Debug)]
pub struct LoadedSequence {
    pub sequence: Sequence,
    pub warnings: Vec<Diagnostic>,
    /// Every file the sequence and its scenes were built from, root first.
    pub files: Vec<SourceFile>,
}

/// What a file describes: one scene, or a sequence of shots of scenes.
#[derive(Debug)]
pub enum Work {
    Scene(Box<Loaded>),
    Sequence(LoadedSequence),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LoadOptions<'a> {
    /// The scene or sequence to load, when the root file has several.
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

/// Loads a scene or a sequence from a file and every file it `use`s.
///
/// # Errors
/// Returns every diagnostic of the load when any of them is an error.
pub fn load_work_file(path: &Path, options: LoadOptions<'_>) -> Result<Work, LoadError> {
    load_work(path, &mut FsSources, options)
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
/// Returns every diagnostic of the load when any of them is an error, or when the file
/// holds a sequence.
pub fn load(
    root: &Path,
    sources: &mut dyn Sources,
    options: LoadOptions<'_>,
) -> Result<Loaded, LoadError> {
    match load_work(root, sources, options)? {
        Work::Scene(loaded) => Ok(*loaded),
        Work::Sequence(loaded) => Err(LoadError {
            diagnostics: vec![Diagnostic {
                severity: Severity::Error,
                file: 0,
                offset: 0,
                message: format!("\"{}\" is a sequence, not a scene", loaded.sequence.name()),
            }],
            files: loaded.files,
        }),
    }
}

/// # Errors
/// Returns every diagnostic of the load when any of them is an error.
pub fn load_work(
    root: &Path,
    sources: &mut dyn Sources,
    options: LoadOptions<'_>,
) -> Result<Work, LoadError> {
    let mut diags = Diagnostics::default();
    let library = Library::load(root, sources, &mut diags);
    let catalog = Catalog::index(&library, &mut diags);
    let built = catalog
        .select(options.scene, &mut diags)
        .and_then(|target| build(target, &catalog, options, &mut diags));
    let files = library.files;
    match (diags.finish(), built) {
        (Ok(warnings), Some(Built::Scene(scene, timeline))) => Ok(Work::Scene(Box::new(Loaded {
            scene: *scene,
            warnings,
            files,
            timeline,
        }))),
        (Ok(warnings), Some(Built::Sequence(sequence))) => Ok(Work::Sequence(LoadedSequence {
            sequence,
            warnings,
            files,
        })),
        // Every path that builds nothing reports an error, so `finish` has already failed.
        (Ok(diagnostics) | Err(diagnostics), _) => Err(LoadError { diagnostics, files }),
    }
}

enum Built {
    Scene(Box<Scene>, Timeline),
    Sequence(Sequence),
}

fn build(
    target: Target<'_>,
    catalog: &Catalog<'_>,
    options: LoadOptions<'_>,
    diags: &mut Diagnostics,
) -> Option<Built> {
    match target {
        Target::Scene(file, node) => {
            diags.in_file(file);
            let overrides = Overrides::Cli(options.params);
            let (spec, timeline) = scene::parse(node, catalog, overrides, diags)?;
            let scene = Scene::new(spec)
                .map_err(|error| diags.error(node.span().offset(), error.to_string()))
                .ok()?;
            Some(Built::Scene(Box::new(scene), timeline))
        }
        Target::Sequence(file, node) => {
            diags.in_file(file);
            if !options.params.is_empty() {
                diags.warning(
                    node.span().offset(),
                    "params are set per shot in a sequence; --param is ignored",
                );
            }
            sequence::parse(node, file, catalog, diags).map(Built::Sequence)
        }
    }
}

/// A scene or a sequence, with the file it was written in.
#[derive(Clone, Copy)]
enum Target<'l> {
    Scene(usize, &'l KdlNode),
    Sequence(usize, &'l KdlNode),
}

/// Palettes, fonts, scenes and sequences by name across every file of a load.
pub(crate) struct Catalog<'l> {
    pub(crate) palettes: BTreeMap<String, NamedPalette>,
    pub(crate) fonts: Fonts,
    /// Each scene with the file it was written in.
    pub(crate) scenes: BTreeMap<&'l str, (usize, &'l KdlNode)>,
    sequences: BTreeMap<&'l str, (usize, &'l KdlNode)>,
    /// The scenes and sequences of the root file, which can be loaded without a name.
    roots: Vec<Target<'l>>,
}

impl<'l> Catalog<'l> {
    fn index(library: &'l Library, diags: &mut Diagnostics) -> Self {
        let mut catalog = Self {
            palettes: BTreeMap::new(),
            fonts: Fonts::new(),
            scenes: BTreeMap::new(),
            sequences: BTreeMap::new(),
            roots: Vec::new(),
        };
        for (file, document) in library.documents() {
            let previous = diags.in_file(file);
            for node in document.nodes() {
                catalog.add(file, node, diags);
            }
            diags.in_file(previous);
        }
        if !catalog.fonts.contains_key(DEFAULT_FONT)
            && let Some(font) = font::default_font()
        {
            catalog.fonts.insert(DEFAULT_FONT.to_owned(), font);
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
            "font" => {
                if let Some((name, font)) = font::parse(node, diags)
                    && self.fonts.insert(name.clone(), font).is_some()
                {
                    diags.error(offset, format!("font \"{name}\" is defined twice"));
                }
            }
            "scene" | "sequence" => self.add_work(file, node, diags),
            other => diags.error(
                offset,
                format!(
                    "unknown top-level node `{other}` (expected `use`, `palette`, `font`, `scene` or `sequence`).{}",
                    suggest::did_you_mean(other, ["use", "palette", "font", "scene", "sequence"])
                ),
            ),
        }
    }

    /// Scenes and sequences share one namespace, so a name picks exactly one.
    fn add_work(&mut self, file: usize, node: &'l KdlNode, diags: &mut Diagnostics) {
        let name = scene_name(node);
        let sequence = node.name().value() == "sequence";
        if self.scenes.contains_key(name) || self.sequences.contains_key(name) {
            diags.error(node.span().offset(), format!("\"{name}\" is defined twice"));
            return;
        }
        let (map, target) = if sequence {
            (&mut self.sequences, Target::Sequence(file, node))
        } else {
            (&mut self.scenes, Target::Scene(file, node))
        };
        map.insert(name, (file, node));
        if file == 0 {
            self.roots.push(target);
        }
    }

    fn select(&self, wanted: Option<&str>, diags: &mut Diagnostics) -> Option<Target<'l>> {
        let chosen = match wanted {
            Some(wanted) => self
                .scenes
                .get(wanted)
                .map(|&(file, node)| Target::Scene(file, node))
                .or_else(|| {
                    let found = self.sequences.get(wanted);
                    found.map(|&(file, node)| Target::Sequence(file, node))
                }),
            None => match self.roots.as_slice() {
                [only] => Some(*only),
                _ => None,
            },
        };
        if chosen.is_none() {
            let names: Vec<&str> = self
                .scenes
                .keys()
                .chain(self.sequences.keys())
                .copied()
                .collect();
            let reason = match wanted {
                Some(wanted) => format!(
                    "no scene or sequence named \"{wanted}\".{}",
                    suggest::did_you_mean(wanted, names.iter().copied())
                ),
                None if self.roots.is_empty() => "the file has no `scene` or `sequence`".to_owned(),
                None => "the file has several scenes or sequences; choose one".to_owned(),
            };
            let previous = diags.in_file(0);
            diags.error(0, format!("{reason} (found: {})", names.join(", ")));
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
