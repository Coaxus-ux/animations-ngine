use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Where documents and settings are read and written; the app never touches the file
/// system itself (D-015). Adapters: the real file system (px-cli), memory (tests).
pub trait FileStore {
    /// # Errors
    /// A readable reason when the file can't be read.
    fn read(&mut self, path: &Path) -> Result<String, String>;

    /// # Errors
    /// A readable reason when the file can't be written.
    fn write(&mut self, path: &Path, text: &str) -> Result<(), String>;

    /// A value that changes whenever the file changes on disk (its modification time),
    /// or `None` when it can't be read.
    fn stamp(&mut self, path: &Path) -> Option<u64>;

    /// Writes an exported image or data file.
    ///
    /// # Errors
    /// A readable reason when the file can't be written; stores that only hold text
    /// refuse by default.
    fn write_bytes(&mut self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        let _ = bytes;
        Err(format!(
            "{}: binary files can't be written here",
            path.display()
        ))
    }
}

/// A store that has nothing and keeps nothing: for restoring a layout and for tests.
pub struct NoFiles;

impl FileStore for NoFiles {
    fn read(&mut self, path: &Path) -> Result<String, String> {
        Err(format!("{}: no files here", path.display()))
    }

    fn write(&mut self, path: &Path, _text: &str) -> Result<(), String> {
        Err(format!("{}: no files here", path.display()))
    }

    fn stamp(&mut self, _path: &Path) -> Option<u64> {
        None
    }
}

/// An in-memory store for tests and examples; the write counter serves as the stamp.
#[derive(Default)]
pub struct MemoryFiles {
    pub files: BTreeMap<PathBuf, String>,
    pub bytes: BTreeMap<PathBuf, Vec<u8>>,
    pub writes: u64,
}

impl MemoryFiles {
    #[must_use]
    pub fn with(mut self, path: impl Into<PathBuf>, text: impl Into<String>) -> Self {
        self.files.insert(path.into(), text.into());
        self
    }
}

impl FileStore for MemoryFiles {
    fn read(&mut self, path: &Path) -> Result<String, String> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| format!("{}: no such file", path.display()))
    }

    fn write(&mut self, path: &Path, text: &str) -> Result<(), String> {
        self.writes += 1;
        self.files.insert(path.to_path_buf(), text.to_owned());
        Ok(())
    }

    fn write_bytes(&mut self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        self.writes += 1;
        self.bytes.insert(path.to_path_buf(), bytes.to_vec());
        Ok(())
    }

    fn stamp(&mut self, path: &Path) -> Option<u64> {
        self.files.contains_key(path).then_some(self.writes)
    }
}
