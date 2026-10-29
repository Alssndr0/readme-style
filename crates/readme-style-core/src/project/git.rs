//! The `origin` remote, read from `.git/config` without running git.

use std::path::Path;

use super::FileSystem;

/// The URL of `remote "origin"` for the repository at `root`, following worktree links.
pub(super) fn origin_url(fs: &dyn FileSystem, root: &Path) -> Option<String> {
    let dot_git = root.join(".git");
    let git_dir = if fs.is_dir(&dot_git) {
        dot_git
    } else {
        let link = fs.read_to_string(&dot_git)?;
        let target = link.strip_prefix("gitdir:")?.trim();
        let gitdir = root.join(target);
        // Worktrees keep their config in the common directory.
        match fs.read_to_string(&gitdir.join("commondir")) {
            Some(common) => gitdir.join(common.trim()),
            None => gitdir,
        }
    };
    remote_url(&fs.read_to_string(&git_dir.join("config"))?, "origin")
}

fn remote_url(config: &str, remote: &str) -> Option<String> {
    let header = format!("[remote \"{remote}\"]");
    let mut inside = false;
    for line in config.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line == header;
        } else if inside
            && let Some((key, value)) = line.split_once('=')
            && key.trim() == "url"
        {
            return Some(value.trim().to_owned());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::MemoryFs;

    const CONFIG: &str = "[core]\n\tbare = false\n[remote \"upstream\"]\n\turl = https://github.com/up/x\n\
                          [remote \"origin\"]\n\turl = git@github.com:me/x.git\n\tfetch = +refs/heads/*\n";

    #[test]
    fn reads_origin() {
        let fs = MemoryFs::new().with("/r/.git/config", CONFIG);
        assert_eq!(
            origin_url(&fs, Path::new("/r")).as_deref(),
            Some("git@github.com:me/x.git")
        );
    }

    #[test]
    fn follows_worktree_links() {
        let fs = MemoryFs::new()
            .with("/w/.git", "gitdir: /r/.git/worktrees/w\n")
            .with("/r/.git/worktrees/w/commondir", "../..\n")
            .with("/r/.git/worktrees/w/../../config", CONFIG);
        assert_eq!(
            origin_url(&fs, Path::new("/w")).as_deref(),
            Some("git@github.com:me/x.git")
        );
    }
}
