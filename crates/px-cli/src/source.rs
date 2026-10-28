use std::path::{Path, PathBuf};

use anyhow::Result;
use px_engine::Scene;
use px_format::{LoadError, LoadOptions, Loaded, SourceFile};

use crate::args::Input;

pub(crate) fn load(input: &Input) -> Result<Scene> {
    Ok(load_tracked(input).0?.scene)
}

/// Loads the input's scene and also returns every file the load read, even when it failed.
pub(crate) fn load_tracked(input: &Input) -> (Result<Loaded>, Vec<PathBuf>) {
    let options = LoadOptions {
        scene: input.scene.as_deref(),
        params: &input.params,
    };
    load_file(&input.file, options)
}

pub(crate) fn load_file(file: &Path, options: LoadOptions<'_>) -> (Result<Loaded>, Vec<PathBuf>) {
    match px_format::load_file(file, options) {
        Ok(loaded) => {
            for warning in &loaded.warnings {
                eprintln!("{}", warning.display(&loaded.files));
            }
            let files = paths(&loaded.files);
            (Ok(loaded), files)
        }
        Err(LoadError { diagnostics, files }) => {
            for diagnostic in &diagnostics {
                eprintln!("{}", diagnostic.display(&files));
            }
            let error = anyhow::anyhow!("{} could not be loaded", file.display());
            (Err(error), paths(&files))
        }
    }
}

fn paths(files: &[SourceFile]) -> Vec<PathBuf> {
    files.iter().map(|file| file.path.clone()).collect()
}
