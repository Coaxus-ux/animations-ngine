use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use kdl::KdlNode;
use px_director::Sequence;
use px_engine::{Detail, Scene};

use crate::diagnostic::{Diagnostic, Diagnostics, LoadError, Severity};
use crate::expand::MotifSignature;
use crate::font::{self, DEFAULT_FONT, Fonts};
use crate::library::Library;
use crate::palette::NamedPalette;
use crate::scene::Overrides;
use crate::source_map::{Origin, PaintNames};
use crate::sources::{FsSources, MemorySources, SourceFile, Sources};
use crate::timeline::Timeline;
use crate::{palettes, scene, sequence, suggest};

const MEMORY_FILE: &str = "scene.kdl";

#[derive(Debug)]
pub struct Loaded {
    pub scene: Scene,
    pub warnings: Vec<Diagnostic>,
    pub files: Vec<SourceFile>,
    pub timeline: Timeline,
    pub origins: Vec<Origin>,
    pub paint_names: PaintNames,
    pub motifs: Vec<MotifSignature>,
    pub assets: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct LoadedSequence {
    pub sequence: Sequence,
    pub frame_budget: Option<Duration>,
    pub warnings: Vec<Diagnostic>,
    pub files: Vec<SourceFile>,
}

#[derive(Debug)]
pub enum Work {
    Scene(Box<Loaded>),
    Sequence(LoadedSequence),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LoadOptions<'a> {
    pub scene: Option<&'a str>,
    pub params: &'a [(String, String)],
    pub detail: Detail,
}

pub fn load_file(path: &Path, options: LoadOptions<'_>) -> Result<Loaded, LoadError> {
    load(path, &mut FsSources, options)
}

pub fn load_work_file(path: &Path, options: LoadOptions<'_>) -> Result<Work, LoadError> {
    load_work(path, &mut FsSources, options)
}

pub fn load_scene(source: &str, options: LoadOptions<'_>) -> Result<Loaded, LoadError> {
    let mut sources = MemorySources::default().with(MEMORY_FILE, source);
    load(Path::new(MEMORY_FILE), &mut sources, options)
}

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
    let Library {
        files,
        motifs,
        assets,
        ..
    } = library;
    match (diags.finish(), built) {
        (Ok(warnings), Some(Built::Scene(built))) => Ok(Work::Scene(Box::new(Loaded {
            scene: built.scene,
            warnings,
            files,
            timeline: built.timeline,
            origins: built.origins,
            paint_names: built.paint_names,
            motifs,
            assets,
        }))),
        (Ok(warnings), Some(Built::Sequence(parsed))) => Ok(Work::Sequence(LoadedSequence {
            sequence: parsed.sequence,
            frame_budget: parsed.frame_budget,
            warnings,
            files,
        })),
        (Ok(diagnostics) | Err(diagnostics), _) => Err(LoadError { diagnostics, files }),
    }
}

enum Built {
    Scene(Box<BuiltScene>),
    Sequence(sequence::Parsed),
}

struct BuiltScene {
    scene: Scene,
    timeline: Timeline,
    origins: Vec<Origin>,
    paint_names: PaintNames,
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
            let parsed = scene::parse(node, catalog, overrides, diags)?;
            let scene = Scene::new(parsed.spec)
                .and_then(|scene| scene.with_detail(options.detail))
                .map_err(|error| diags.error(node.span().offset(), error.to_string()))
                .ok()?;
            Some(Built::Scene(Box::new(BuiltScene {
                scene,
                timeline: parsed.timeline,
                origins: parsed.origins,
                paint_names: parsed.paint_names,
            })))
        }
        Target::Sequence(file, node) => {
            diags.in_file(file);
            if !options.params.is_empty() {
                diags.warning(
                    node.span().offset(),
                    "params are set per shot in a sequence; --param is ignored",
                );
            }
            let parsed = sequence::parse(node, file, catalog, diags)?;
            parsed
                .sequence
                .with_detail(options.detail)
                .map_err(|error| diags.error(node.span().offset(), error.to_string()))
                .ok()
                .map(|sequence| Built::Sequence(sequence::Parsed { sequence, ..parsed }))
        }
    }
}

#[derive(Clone, Copy)]
enum Target<'l> {
    Scene(usize, &'l KdlNode),
    Sequence(usize, &'l KdlNode),
}

pub(crate) struct Catalog<'l> {
    pub(crate) palettes: BTreeMap<String, NamedPalette>,
    palette_nodes: Vec<(usize, &'l KdlNode)>,
    pub(crate) fonts: Fonts,
    pub(crate) scenes: BTreeMap<&'l str, (usize, &'l KdlNode)>,
    sequences: BTreeMap<&'l str, (usize, &'l KdlNode)>,
    roots: Vec<Target<'l>>,
}

impl<'l> Catalog<'l> {
    fn index(library: &'l Library, diags: &mut Diagnostics) -> Self {
        let mut catalog = Self {
            palettes: BTreeMap::new(),
            palette_nodes: Vec::new(),
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
        catalog.palettes = palettes::build(&catalog.palette_nodes, diags);
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
            "palette" => self.palette_nodes.push((file, node)),
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
                    "unknown top-level node `{other}` (expected `use`, `palette`, `font`, `motif`, `scene` or `sequence`).{}",
                    suggest::did_you_mean(other, ["use", "palette", "font", "motif", "scene", "sequence"])
                ),
            ),
        }
    }

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
