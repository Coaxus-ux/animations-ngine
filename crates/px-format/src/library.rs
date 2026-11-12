use std::path::{Path, PathBuf};

use kdl::{KdlDocument, KdlError, KdlNode};

use crate::diagnostic::Diagnostics;
use crate::expand::{self, MotifSignature};
use crate::pixels;
use crate::sources::{self, SourceFile, Sources};
use crate::values;

pub(crate) struct Library {
    pub(crate) files: Vec<SourceFile>,
    pub(crate) motifs: Vec<MotifSignature>,
    pub(crate) assets: Vec<PathBuf>,
    documents: Vec<Option<KdlDocument>>,
}

impl Library {
    pub(crate) fn load(root: &Path, sources: &mut dyn Sources, diags: &mut Diagnostics) -> Self {
        let mut library = Self {
            files: Vec::new(),
            motifs: Vec::new(),
            assets: Vec::new(),
            documents: Vec::new(),
        };
        let mut loader = Loader {
            sources,
            stack: Vec::new(),
        };
        loader.visit(&mut library, &sources::normalize(root), None, diags);
        library.assets = pixels::inline(
            &mut library.documents,
            &library.files,
            loader.sources,
            diags,
        );
        library.motifs = expand::run(&mut library.documents, &library.files, diags);
        library
    }

    pub(crate) fn documents(&self) -> impl Iterator<Item = (usize, &KdlDocument)> {
        self.documents
            .iter()
            .enumerate()
            .filter_map(|(file, document)| document.as_ref().map(|document| (file, document)))
    }
}

struct Loader<'s> {
    sources: &'s mut dyn Sources,
    stack: Vec<PathBuf>,
}

type Origin = (usize, usize);

impl Loader<'_> {
    fn visit(
        &mut self,
        library: &mut Library,
        path: &Path,
        origin: Option<Origin>,
        diags: &mut Diagnostics,
    ) {
        if let Some(start) = self.stack.iter().position(|open| open == path) {
            let chain: Vec<String> = self.stack[start..]
                .iter()
                .chain([&path.to_path_buf()])
                .map(|open| open.display().to_string())
                .collect();
            report(origin, diags, format!("`use` cycle: {}", chain.join(" → ")));
            return;
        }
        if library.files.iter().any(|file| file.path == path) {
            return;
        }
        let text = match self.sources.read(path) {
            Ok(text) => text,
            Err(reason) => {
                let message = format!("cannot read {}: {reason}", path.display());
                report(origin, diags, message);
                return;
            }
        };
        let file = library.files.len();
        let document = KdlDocument::parse_v2(&text);
        library.files.push(SourceFile {
            path: path.to_path_buf(),
            text,
        });
        let previous = diags.in_file(file);
        let document = document.map_err(|error| syntax_errors(&error, diags)).ok();
        let uses = document.as_ref().map(uses).unwrap_or_default();
        let targets: Vec<(PathBuf, usize)> = uses
            .into_iter()
            .filter_map(|node| {
                let entry = node.entries().first();
                let target = entry.and_then(|entry| values::string(entry, diags));
                if target.is_none() && entry.is_none() {
                    diags.error(node.span().offset(), "`use` needs a file path");
                }
                target.map(|target| (sources::resolve(path, target), node.span().offset()))
            })
            .collect();
        library.documents.push(document);
        self.stack.push(path.to_path_buf());
        for (target, offset) in targets {
            self.visit(library, &target, Some((file, offset)), diags);
        }
        self.stack.pop();
        diags.in_file(previous);
    }
}

fn uses(document: &KdlDocument) -> Vec<&KdlNode> {
    document
        .nodes()
        .iter()
        .filter(|node| node.name().value() == "use")
        .collect()
}

fn report(origin: Option<Origin>, diags: &mut Diagnostics, message: String) {
    let (file, offset) = origin.unwrap_or((0, 0));
    let previous = diags.in_file(file);
    diags.error(offset, message);
    diags.in_file(previous);
}

fn syntax_errors(error: &KdlError, diags: &mut Diagnostics) {
    let before = diags.has_errors();
    for diagnostic in &error.diagnostics {
        let message = diagnostic
            .message
            .clone()
            .unwrap_or_else(|| "invalid KDL syntax".to_owned());
        diags.error(diagnostic.span.offset(), message);
    }
    if !before && !diags.has_errors() {
        diags.error(0, "invalid KDL syntax");
    }
}
