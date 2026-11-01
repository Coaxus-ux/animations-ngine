use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub trait Sources {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFile {
    pub path: PathBuf,
    pub text: String,
}

pub(crate) fn resolve(from: &Path, target: &str) -> PathBuf {
    let base = from.parent().unwrap_or_else(|| Path::new(""));
    normalize(&base.join(target))
}

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
