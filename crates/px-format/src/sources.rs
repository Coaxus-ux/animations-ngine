use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

/// Where scene files come from: the file system for the CLI, memory for tests.
pub trait Sources {
    /// # Errors
    /// Returns a readable reason when the file can't be read.
    fn read(&mut self, path: &Path) -> Result<String, String>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FsSources;

impl Sources for FsSources {
    fn read(&mut self, path: &Path) -> Result<String, String> {
        fs::read_to_string(path).map_err(|error| error.to_string())
    }
}

#[derive(Clone, Debug, Default)]
pub struct MemorySources {
    files: BTreeMap<PathBuf, String>,
}

impl MemorySources {
    #[must_use]
    pub fn with(mut self, path: impl Into<PathBuf>, text: impl Into<String>) -> Self {
        self.files.insert(normalize(&path.into()), text.into());
        self
    }
}

impl Sources for MemorySources {
    fn read(&mut self, path: &Path) -> Result<String, String> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| "no such file".to_owned())
    }
}

/// One file read during a load, kept so diagnostics can show its lines.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFile {
    pub path: PathBuf,
    pub text: String,
}

/// `target` as written in a `use` inside `from`: relative to that file's folder.
pub(crate) fn resolve(from: &Path, target: &str) -> PathBuf {
    let base = from.parent().unwrap_or_else(|| Path::new(""));
    normalize(&base.join(target))
}

/// Removes `.` and folds `..` without touching the file system, so one file always has
/// one name.
pub(crate) fn normalize(path: &Path) -> PathBuf {
    let mut parts: Vec<Component<'_>> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir if matches!(parts.last(), Some(Component::Normal(_))) => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.iter().collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::resolve;

    #[test]
    fn uses_resolve_next_to_the_importing_file() {
        let from = Path::new("examples/town.kdl");
        assert_eq!(
            resolve(from, "parts/walker.kdl"),
            Path::new("examples/parts/walker.kdl")
        );
        assert_eq!(
            resolve(Path::new("examples/parts/walker.kdl"), "../palette.kdl"),
            Path::new("examples/palette.kdl")
        );
        assert_eq!(
            resolve(Path::new("town.kdl"), "./a.kdl"),
            Path::new("a.kdl")
        );
    }
}
