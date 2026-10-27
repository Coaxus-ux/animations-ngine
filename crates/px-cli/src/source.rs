use std::fs;

use anyhow::{Context, Result, bail};
use px_engine::Scene;
use px_format::{LoadError, load_scene};

use crate::args::Input;

/// Reads and loads the scene named by `input`, printing warnings to stderr.
pub(crate) fn load(input: &Input) -> Result<Scene> {
    let path = input.file.display().to_string();
    let source = fs::read_to_string(&input.file).with_context(|| format!("cannot read {path}"))?;
    match load_scene(&source, input.scene.as_deref()) {
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
