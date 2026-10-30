use std::path::Path;

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
}
