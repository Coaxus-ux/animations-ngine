use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Stamp {
    modified: Option<SystemTime>,
    len: u64,
}

#[derive(Debug)]
pub(crate) struct FileWatch {
    path: PathBuf,
    seen: Option<Stamp>,
}

impl FileWatch {
    pub(crate) fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            seen: None,
        }
    }

    /// True the first time, then whenever the file's modification time or size changes.
    pub(crate) fn changed(&mut self) -> bool {
        let Ok(metadata) = fs::metadata(&self.path) else {
            return false;
        };
        let stamp = Some(Stamp {
            modified: metadata.modified().ok(),
            len: metadata.len(),
        });
        let changed = stamp != self.seen;
        self.seen = stamp;
        changed
    }
}

#[cfg(test)]
#[path = "file_watch_tests.rs"]
mod tests;
