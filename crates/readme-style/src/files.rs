//! Finding READMEs and writing them back safely.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const README_NAMES: &[&str] = &[
    "README.md",
    "readme.md",
    "Readme.md",
    "README.markdown",
    "readme.markdown",
];

/// The README a path names: the file itself, or the README inside a folder.
pub(crate) fn find_readme(path: &Path) -> Option<PathBuf> {
    if path.is_dir() {
        return README_NAMES
            .iter()
            .map(|name| path.join(name))
            .find(|p| p.is_file());
    }
    path.is_file().then(|| path.to_path_buf())
}

/// Replace `path` with `content` without ever leaving a half-written file, keeping its permissions.
pub(crate) fn write_atomic(path: &Path, content: &str) -> io::Result<()> {
    let name = path
        .file_name()
        .map_or_else(|| "README".into(), |n| n.to_string_lossy().into_owned());
    let temp = path.with_file_name(format!(".{name}.readme-style.tmp"));
    fs::write(&temp, content)?;
    let result = fs::metadata(path)
        .and_then(|meta| fs::set_permissions(&temp, meta.permissions()))
        .and_then(|()| fs::rename(&temp, path));
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}
