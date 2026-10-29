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

/// Replace the README at `path` with `content`, never leaving a half-written file.
///
/// A symbolic link is followed, so the file it points to is updated and the link stays. A file
/// with several hard links is rewritten in place, since replacing it would detach the others.
pub(crate) fn write_readme(path: &Path, content: &str) -> io::Result<()> {
    let target = fs::canonicalize(path)?;
    if hard_linked(&target)? {
        return fs::write(&target, content);
    }
    let name = target
        .file_name()
        .map_or_else(|| "README".into(), |n| n.to_string_lossy().into_owned());
    let temp = target.with_file_name(format!(".{name}.readme-style.tmp"));
    fs::write(&temp, content)?;
    let result = fs::metadata(&target)
        .and_then(|meta| fs::set_permissions(&temp, meta.permissions()))
        .and_then(|()| fs::rename(&temp, &target));
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(unix)]
fn hard_linked(path: &Path) -> io::Result<bool> {
    use std::os::unix::fs::MetadataExt as _;
    Ok(fs::metadata(path)?.nlink() > 1)
}

#[cfg(not(unix))]
fn hard_linked(_: &Path) -> io::Result<bool> {
    Ok(false)
}
