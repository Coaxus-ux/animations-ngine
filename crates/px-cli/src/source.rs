use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use px_engine::Scene;
use px_format::{LoadError, LoadOptions, load_scene};

use crate::args::Input;

pub(crate) fn load(input: &Input) -> Result<Scene> {
    let options = LoadOptions {
        scene: input.scene.as_deref(),
        params: &input.params,
    };
    load_file(&input.file, options)
}

pub(crate) fn load_file(file: &Path, options: LoadOptions<'_>) -> Result<Scene> {
    let path = file.display().to_string();
    let source = fs::read_to_string(file).with_context(|| format!("cannot read {path}"))?;
    match load_scene(&source, options) {
        Ok(loaded) => {
            for warning in &loaded.warnings {
                eprintln!("{}", warning.render(&path, &source));
            }
            Ok(loaded.scene)
        }
        Err(LoadError { diagnostics }) => {
            for diagnostic in &diagnostics {
                eprintln!("{}", diagnostic.render(&path, &source));
            }
            bail!("{path} could not be loaded")
        }
    }
}
