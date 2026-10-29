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

/// The file a README path names, after symbolic links, provided no link on the way could point
/// readme-style at a file it must not read, print in a diff, send to a model or rewrite.
///
/// - The README itself, when it is a link, must lead to a Markdown file (by its extension, or a
///   plain `README`) in its own folder or below, outside any `.git`. A repository's README linking
///   to `../.env`, to a key in your home folder, or even to a `.env` you created next to it, is
///   refused.
/// - A folder link on the way must stay in the repository it sits in (the nearest folder holding
///   `.git` or `.github`), outside any `.git`, so a cloned repository cannot point a folder at
///   another one. Folder links outside any repository are your own, such as a projects folder
///   linked from home, and are followed.
///
/// Two harmless setups are refused because they look like the ones above: a package README linking
/// to its repository's README above it, and a link in a home folder kept in git to a project
/// elsewhere. The message names the real path to use instead.
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
                let refusal = if index == last {
                    let folder = fs::canonicalize(&real)?;
                    if !inside(&resolved, &folder) {
                        Some(format!("outside its folder {}", folder.display()))
                    } else if !is_markdown(&resolved) {
                        Some("which is not a Markdown file".to_owned())
                    } else {
                        None
                    }
                } else if let Some(repository) = repository_of(&real) {
                    // Compare real paths: on Windows they carry the same `\\?\` prefix.
                    let repository = fs::canonicalize(&repository)?;
                    (!inside(&resolved, &repository))
                        .then(|| format!("outside the repository at {}", repository.display()))
                } else {
                    None
                };
                if let Some(refusal) = refusal {
                    let target = fs::canonicalize(path).unwrap_or_else(|_| resolved.clone());
                    return Err(io::Error::other(format!(
                        "{} leads to {}, {refusal}; if that is intended, run readme-style on {}",
                        next.display(),
                        resolved.display(),
                        target.display()
                    )));
                }
                real = resolved;
            }
        }
    }
    Ok(real)
}

/// Whether `path` is under `folder` and outside any `.git` there.
fn inside(path: &Path, folder: &Path) -> bool {
    path.strip_prefix(folder)
        .is_ok_and(|rest| rest.components().all(|c| c.as_os_str() != ".git"))
}

/// Whether a file name says Markdown: a Markdown extension, or a plain `README`.
fn is_markdown(path: &Path) -> bool {
    const EXTENSIONS: &[&str] = &["md", "markdown", "mdown", "mkdn", "mkd", "mdwn", "mdx"];
    match path.extension().and_then(|e| e.to_str()) {
        Some(extension) => EXTENSIONS.iter().any(|m| extension.eq_ignore_ascii_case(m)),
        None => path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case("readme")),
    }
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
