use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use px_editor::ports::FileStore;

pub(crate) struct DiskFiles;

impl FileStore for DiskFiles {
    fn read(&mut self, path: &Path) -> Result<String, String> {
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))
    }

    fn write(&mut self, path: &Path, text: &str) -> Result<(), String> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
        }
        fs::write(path, text).map_err(|error| format!("{}: {error}", path.display()))
    }

    fn read_bytes(&mut self, path: &Path) -> Result<Vec<u8>, String> {
        fs::read(path).map_err(|error| format!("{}: {error}", path.display()))
    }

    fn write_bytes(&mut self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
        }
        fs::write(path, bytes).map_err(|error| format!("{}: {error}", path.display()))
    }

    fn stamp(&mut self, path: &Path) -> Option<u64> {
        let modified = fs::metadata(path).ok()?.modified().ok()?;
        let since = modified.duration_since(UNIX_EPOCH).ok()?;
        u64::try_from(since.as_nanos()).ok()
    }
}
