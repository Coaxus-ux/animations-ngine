use std::path::{Path, PathBuf};

use anyhow::Result;
use px_format::{Diagnostic, LoadError, LoadOptions, Loaded, SourceFile, Work};

use crate::args::Input;

/// Loads the input's scene and also returns every file the load read, even when it failed.
pub(crate) fn load_tracked(input: &Input) -> (Result<Loaded>, Vec<PathBuf>) {
    load_file(&input.file, options(input))
}

pub(crate) fn load_file(file: &Path, options: LoadOptions<'_>) -> (Result<Loaded>, Vec<PathBuf>) {
    match px_format::load_file(file, options) {
        Ok(loaded) => {
            print(&loaded.warnings, &loaded.files);
            let files = paths(&loaded.files);
            (Ok(loaded), files)
        }
        Err(error) => failed(file, error),
    }
}

pub(crate) fn load_work(input: &Input) -> Result<Work> {
    load_work_file(&input.file, options(input))
}

pub(crate) fn load_work_file(file: &Path, options: LoadOptions<'_>) -> Result<Work> {
    match px_format::load_work_file(file, options) {
        Ok(work) => {
            let (warnings, files) = match &work {
                Work::Scene(loaded) => (&loaded.warnings, &loaded.files),
                Work::Sequence(loaded) => (&loaded.warnings, &loaded.files),
            };
            print(warnings, files);
            Ok(work)
        }
        Err(error) => failed(file, error).0,
    }
}

fn options(input: &Input) -> LoadOptions<'_> {
    LoadOptions {
        scene: input.scene.as_deref(),
        params: &input.params,
    }
}

fn print(diagnostics: &[Diagnostic], files: &[SourceFile]) {
    for diagnostic in diagnostics {
        eprintln!("{}", diagnostic.display(files));
    }
}

fn failed<T>(
    file: &Path,
    LoadError { diagnostics, files }: LoadError,
) -> (Result<T>, Vec<PathBuf>) {
    print(&diagnostics, &files);
    let error = anyhow::anyhow!("{} could not be loaded", file.display());
    (Err(error), paths(&files))
}

fn paths(files: &[SourceFile]) -> Vec<PathBuf> {
    files.iter().map(|file| file.path.clone()).collect()
}
