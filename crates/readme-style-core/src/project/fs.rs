//! File access for project inspection, abstracted so it can run on a real disk or in memory.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The file operations project inspection needs. Paths are used as given.
pub trait FileSystem {
    /// The file's text, or `None` when it does not exist or is not UTF-8.
    fn read_to_string(&self, path: &Path) -> Option<String>;
    fn is_file(&self, path: &Path) -> bool;
    fn is_dir(&self, path: &Path) -> bool;
    /// Entries of a directory, sorted by path. Empty when it is not a directory.
    fn list_dir(&self, path: &Path) -> Vec<PathBuf>;

    fn exists(&self, path: &Path) -> bool {
        self.is_file(path) || self.is_dir(path)
    }
}

/// The real file system.
#[derive(Debug, Clone, Copy, Default)]
pub struct OsFs;

impl FileSystem for OsFs {
    fn read_to_string(&self, path: &Path) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn list_dir(&self, path: &Path) -> Vec<PathBuf> {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(path)
            .map(|dir| dir.filter_map(Result::ok).map(|e| e.path()).collect())
            .unwrap_or_default();
        entries.sort();
        entries
    }
}

/// An in-memory file tree, for tests and for hosts without a disk.
#[derive(Debug, Clone, Default)]
pub struct MemoryFs {
    files: BTreeMap<PathBuf, String>,
}

impl MemoryFs {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a file; its parent directories exist implicitly.
    pub fn insert(&mut self, path: impl Into<PathBuf>, content: impl Into<String>) -> &mut Self {
        self.files.insert(path.into(), content.into());
        self
    }

    #[must_use]
    pub fn with(mut self, path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        self.insert(path, content);
        self
    }
}

impl FileSystem for MemoryFs {
    fn read_to_string(&self, path: &Path) -> Option<String> {
        self.files.get(path).cloned()
    }

    fn is_file(&self, path: &Path) -> bool {
        self.files.contains_key(path)
    }

    fn is_dir(&self, path: &Path) -> bool {
        self.files
            .keys()
            .any(|file| file.starts_with(path) && file != path)
    }

    fn list_dir(&self, path: &Path) -> Vec<PathBuf> {
        let mut entries: Vec<PathBuf> = self
            .files
            .keys()
            .filter_map(|file| {
                let rest = file.strip_prefix(path).ok()?;
                let first = rest.components().next()?;
                Some(path.join(first))
            })
            .collect();
        entries.sort();
        entries.dedup();
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_fs_has_implicit_directories() {
        let fs = MemoryFs::new().with("/p/a/b.txt", "x").with("/p/c.txt", "y");
        assert!(fs.is_dir(Path::new("/p/a")));
        assert!(fs.is_file(Path::new("/p/c.txt")));
        assert_eq!(
            fs.list_dir(Path::new("/p")),
            [PathBuf::from("/p/a"), PathBuf::from("/p/c.txt")]
        );
        assert!(fs.list_dir(Path::new("/p/c.txt")).is_empty());
    }
}
