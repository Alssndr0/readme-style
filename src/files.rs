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

/// The file a README path names, after symbolic links, provided it lies in the repository the
/// path itself is in (and not inside `.git`). A link in a cloned repository, to the README or
/// to a folder above it, must not point readme-style at a file elsewhere: not to rewrite it,
/// and not to print it in a diff.
pub(crate) fn resolve_readme(path: &Path) -> io::Result<PathBuf> {
    let target = fs::canonicalize(path)?;
    let lexical = std::path::absolute(path)?;
    let folder = lexical.parent().unwrap_or(Path::new("/"));
    let root = fs::canonicalize(find_root(&OsFs, folder))?;
    let inside = target.strip_prefix(&root).is_ok_and(|rest| {
        rest.components()
            .next()
            .is_none_or(|first| first.as_os_str() != ".git")
    });
    if inside {
        Ok(target)
    } else {
        Err(io::Error::other(format!(
            "it leads to {}, outside the repository at {}; run readme-style on that file directly if you mean to",
            target.display(),
            root.display()
        )))
    }
}

/// Replace the README at `path` with `content`; `original` is what was read from it.
///
/// - Only a file [`resolve_readme`] accepts is written, and only if the file itself is writable.
/// - The content first goes to a temporary file with a fresh, unpredictable name (never an
///   existing path, so nothing planted there is followed). A full disk therefore stops the write
///   before the README is touched.
/// - That file then replaces the README atomically, with its permissions, owner and group. When
///   it cannot take them (another owner, a group we are not in), or the README has other hard
///   links that a replacement would detach, the README is rewritten in place instead, and put
///   back as it was if that fails.
pub(crate) fn write_readme(path: &Path, content: &str, original: &str) -> io::Result<()> {
    let target = resolve_readme(path)?;
    drop(OpenOptions::new().write(true).open(&target)?);
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
    if hard_linked(&target)? || !takes_ownership(temp.as_file(), &target)? {
        // The space it took is now free for the rewrite.
        drop(temp);
        return rewrite_in_place(&target, content, original);
    }
    temp.as_file()
        .set_permissions(fs::metadata(&target)?.permissions())?;
    temp.persist(&target).map_err(|e| e.error)?;
    Ok(())
}

/// Whether `temp` has, or could be given, the owner and group of `target`.
#[cfg(unix)]
fn takes_ownership(temp: &fs::File, target: &Path) -> io::Result<bool> {
    use std::os::unix::fs::{MetadataExt as _, fchown};
    let (ours, theirs) = (temp.metadata()?, fs::metadata(target)?);
    if ours.uid() != theirs.uid() {
        return Ok(false);
    }
    Ok(ours.gid() == theirs.gid() || fchown(temp, None, Some(theirs.gid())).is_ok())
}

#[cfg(not(unix))]
fn takes_ownership(_: &fs::File, _: &Path) -> io::Result<bool> {
    Ok(true)
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
