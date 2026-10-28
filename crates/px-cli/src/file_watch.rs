use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Stamp {
    modified: Option<SystemTime>,
    len: u64,
}

fn stamp(path: &Path) -> Option<Stamp> {
    let metadata = fs::metadata(path).ok()?;
    Some(Stamp {
        modified: metadata.modified().ok(),
        len: metadata.len(),
    })
}

/// Watches the root scene file and every file it uses.
#[derive(Debug)]
pub(crate) struct FileWatch {
    files: Vec<(PathBuf, Option<Stamp>)>,
}

impl FileWatch {
    pub(crate) fn new(path: &Path) -> Self {
        Self {
            files: vec![(path.to_path_buf(), None)],
        }
    }

    /// Replaces the watched set after a load; files new to the set start from their
    /// current state, so tracking them doesn't count as a change.
    pub(crate) fn track(&mut self, paths: &[PathBuf]) {
        let files = paths
            .iter()
            .map(|path| {
                let seen = self
                    .files
                    .iter()
                    .find(|(known, _)| known == path)
                    .map_or_else(|| stamp(path), |(_, seen)| *seen);
                (path.clone(), seen)
            })
            .collect();
        self.files = files;
    }

    /// True the first time, then whenever a watched file's modification time or size
    /// changes. A file that can't be read doesn't count as a change.
    pub(crate) fn changed(&mut self) -> bool {
        let mut changed = false;
        for (path, seen) in &mut self.files {
            let Some(now) = stamp(path) else {
                continue;
            };
            if Some(now) != *seen {
                *seen = Some(now);
                changed = true;
            }
        }
        changed
    }
}

#[cfg(test)]
#[path = "file_watch_tests.rs"]
mod tests;
