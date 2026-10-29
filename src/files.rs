//! Finding READMEs and writing them back safely.

use std::fs::{self, OpenOptions};
use std::io::{self, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

use readme_style_core::project::{OsFs, find_root};

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

/// Replace the README at `path` with `content`; `original` is what was read from it.
///
/// - A symbolic link is followed, so the file it points to is updated and the link stays, but
///   only when that file is in the link's repository: a README in a cloned repository must not
///   be able to aim the write at a file elsewhere.
/// - The content first goes to a temporary file with a fresh, unpredictable name (never an
///   existing path, so nothing planted there is followed). A full disk or a file size limit
///   therefore stops the write before the README is touched.
/// - A file with several hard links is then rewritten in place, since replacing it would detach
///   the others, and put back as it was if that fails. Any other file is replaced atomically by
///   renaming the temporary file over it.
pub(crate) fn write_readme(path: &Path, content: &str, original: &str) -> io::Result<()> {
    let target = fs::canonicalize(path)?;
    if fs::symlink_metadata(path)?.file_type().is_symlink() {
        let folder = path.parent().filter(|p| !p.as_os_str().is_empty());
        let root = find_root(&OsFs, &fs::canonicalize(folder.unwrap_or(Path::new(".")))?);
        if !target.starts_with(&root) {
            return Err(io::Error::other(format!(
                "it links to {}, outside {}; style that file directly if you mean to",
                target.display(),
                root.display()
            )));
        }
    }
    let folder = target
        .parent()
        .ok_or_else(|| io::Error::other("the README has no parent folder"))?;
    let name = target
        .file_name()
        .map_or_else(|| "README".into(), |n| n.to_string_lossy().into_owned());
    let mut temp = tempfile::Builder::new()
        .prefix(&format!(".{name}."))
        .suffix(".readme-style.tmp")
        .tempfile_in(folder)?;
    temp.write_all(content.as_bytes())?;
    temp.as_file().sync_all()?;
    if hard_linked(&target)? {
        // The space it took is now free for the rewrite.
        drop(temp);
        return rewrite_in_place(&target, content, original);
    }
    temp.as_file()
        .set_permissions(fs::metadata(&target)?.permissions())?;
    temp.persist(&target).map_err(|e| e.error)?;
    Ok(())
}

/// Overwrite `target` with `content`, restoring `original` if the write fails part way.
fn rewrite_in_place(target: &Path, content: &str, original: &str) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).open(target)?;
    let written = file
        .write_all(content.as_bytes())
        .and_then(|()| file.set_len(content.len() as u64))
        .and_then(|()| file.sync_all());
    let Err(error) = written else {
        return Ok(());
    };
    let restored = file
        .seek(SeekFrom::Start(0))
        .and_then(|_| file.write_all(original.as_bytes()))
        .and_then(|()| file.set_len(original.len() as u64));
    Err(match restored {
        Ok(()) => error,
        Err(again) => io::Error::other(format!("{error}; putting the original back failed too ({again})")),
    })
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
