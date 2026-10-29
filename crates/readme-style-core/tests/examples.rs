//! The gallery in `examples/` doubles as snapshot tests: every example README, styled with
//! every built-in theme, must equal its checked-in `README.<theme>.md`.
//! Regenerate with `cargo xtask examples` and review the diff.

use std::fs;
use std::path::{Path, PathBuf};

use readme_style_core::config;
use readme_style_core::project::{OsFs, inspect};
use readme_style_core::style;
use readme_style_core::theme::Themes;

fn examples() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut dirs: Vec<PathBuf> = fs::read_dir(&root)
        .expect("examples folder")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.join("README.md").is_file())
        .collect();
    dirs.sort();
    dirs
}

#[test]
fn gallery_is_current() {
    let dirs = examples();
    assert!(dirs.len() >= 3, "expected the example projects");
    let mut stale = Vec::new();
    for dir in dirs {
        let readme = fs::canonicalize(dir.join("README.md")).expect("readme");
        let text = fs::read_to_string(&readme).expect("readable");
        let config = config::discover(&OsFs, &readme).expect("valid config");
        let facts = inspect(&OsFs, &readme, &config.project_overrides());
        for name in Themes::builtin().names() {
            let theme = config.theme(name).expect("theme");
            let output = style(&text, &theme, &facts).expect("styles").output;
            let target = dir.join(format!("README.{name}.md"));
            if fs::read_to_string(&target).ok().as_deref() != Some(output.as_str()) {
                stale.push(target.display().to_string());
            }
            let again = style(&output, &theme, &facts).expect("styles again");
            assert!(!again.changed, "{} is not stable", target.display());
        }
    }
    assert!(stale.is_empty(), "out of date, run `cargo xtask examples`: {stale:?}");
}
