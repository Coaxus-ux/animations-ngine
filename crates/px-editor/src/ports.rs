use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub trait FileStore {
    fn read(&mut self, path: &Path) -> Result<String, String>;

    fn write(&mut self, path: &Path, text: &str) -> Result<(), String>;

    fn stamp(&mut self, path: &Path) -> Option<u64>;

    fn read_bytes(&mut self, path: &Path) -> Result<Vec<u8>, String> {
        Err(format!(
            "{}: binary files can't be read here",
            path.display()
        ))
    }

    fn write_bytes(&mut self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        let _ = bytes;
        Err(format!(
            "{}: binary files can't be written here",
            path.display()
        ))
    }
}

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

    fn read_bytes(&mut self, path: &Path) -> Result<Vec<u8>, String> {
        self.bytes
            .get(path)
            .cloned()
            .ok_or_else(|| format!("{}: no such file", path.display()))
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
