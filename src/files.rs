//! Finding READMEs and writing them back safely.

use std::fs::{self, OpenOptions};
use std::io::{self, Seek as _, SeekFrom, Write as _};
use std::path::{Component, Path, PathBuf};

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

/// The file a README path names, after symbolic links, provided no link on the way leaves the
/// repository it sits in or points into a `.git` folder. A link inside a cloned repository, to
/// the README or to a folder above it, must not point readme-style at a file elsewhere: not to
/// rewrite it, not to print it in a diff, not to send it to a model.
///
/// Outside any repository (a downloaded archive, a folder of notes), the README itself may only
/// link within its own folder, while folder links on the way are the user's own, such as a
/// projects folder linked from home, and are followed.
///
/// One setup is refused although it is harmless: a link in a folder that is itself a
/// repository (a home folder kept in git) to a project elsewhere. It cannot be told apart from
/// a repository linking to another one; the message names the real path to use instead.
pub(crate) fn resolve_readme(path: &Path) -> io::Result<PathBuf> {
    let absolute = std::path::absolute(path)?;
    let last = absolute.components().count().saturating_sub(1);
    let mut real = PathBuf::new();
    for (index, component) in absolute.components().enumerate() {
        match component {
            Component::Prefix(_) | Component::RootDir => real.push(component),
            Component::CurDir => {}
            Component::ParentDir => {
                real.pop();
            }
            Component::Normal(name) => {
                let next = real.join(name);
                if !fs::symlink_metadata(&next)?.file_type().is_symlink() {
                    real = next;
                    continue;
                }
                let resolved = fs::canonicalize(&next)?;
                let boundary = match repository_of(&real) {
                    Some(repository) => Some(("the repository at", repository)),
                    None if index == last => Some(("its folder", real.clone())),
                    None => None,
                };
                if let Some((what, boundary)) = boundary {
                    // Compare real paths: on Windows they carry the same `\\?\` prefix.
                    let boundary = fs::canonicalize(&boundary)?;
                    let inside = resolved
                        .strip_prefix(&boundary)
                        .is_ok_and(|rest| rest.components().all(|c| c.as_os_str() != ".git"));
                    if !inside {
                        let target = fs::canonicalize(path).unwrap_or_else(|_| resolved.clone());
                        return Err(io::Error::other(format!(
                            "{} leads to {}, outside {what} {}; if that is intended, run readme-style on {}",
                            next.display(),
                            resolved.display(),
                            boundary.display(),
                            target.display()
                        )));
                    }
                }
                real = resolved;
            }
        }
    }
    Ok(real)
}

/// The repository a real (link-free) folder is in: the nearest folder holding `.git` or
/// `.github`.
fn repository_of(folder: &Path) -> Option<PathBuf> {
    folder
        .ancestors()
        .find(|dir| dir.join(".git").exists() || dir.join(".github").is_dir())
        .map(Path::to_path_buf)
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
    let temp = tempfile::Builder::new()
        .prefix(&format!(".{name}."))
        .suffix(".readme-style.tmp")
        .tempfile_in(folder);
    let mut temp = match temp {
        Ok(temp) => temp,
        // A writable README in a folder that is not: it can only be rewritten in place.
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            return rewrite_in_place(&target, content, original);
        }
        Err(e) => return Err(e),
    };
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
