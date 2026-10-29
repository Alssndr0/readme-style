//! Logo discovery: `logo.svg`, `logo-dark.svg` and similar names in the usual folders.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use super::FileSystem;

/// Folders searched, in order, relative to the README.
pub(super) const FOLDERS: &[&str] = &[
    ".",
    "assets",
    "asset",
    "images",
    "image",
    "img",
    "imgs",
    "media",
    "static",
    "resources",
    "art",
    "branding",
    "brand",
    "logo",
    "logos",
    ".github",
    ".github/assets",
    ".github/images",
    "docs",
    "docs/assets",
    "docs/images",
    "docs/img",
    "docs/static",
    "docs/_static",
    "docs/source/_static",
];

static LOGO_FILE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?P<stem>[\w.-]*logo[\w.-]*)\.(?P<ext>svg|png|webp|jpe?g)$").expect("valid regex")
});

static IMAGE_FILE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(?P<stem>.+)\.(?:svg|png|webp|jpe?g)$").expect("valid regex"));

const DARK: &[&str] = &[
    "dark", "white", "inverted", "negative", "night", "ondark", "fordark",
];
const LIGHT: &[&str] = &["light", "black", "day", "positive", "onlight", "forlight"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Variant {
    Default,
    Light,
    Dark,
}

/// Split a file stem into its base and its variant: `logo-dark` is (`logo`, dark).
pub(super) fn variant(stem: &str) -> (String, Variant) {
    let lower = stem.to_ascii_lowercase();
    let mut tokens: Vec<String> = lower
        .split(['-', '_', '.'])
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect();
    if let Some(i) = tokens
        .windows(2)
        .position(|w| matches!(w[0].as_str(), "on" | "for") && matches!(w[1].as_str(), "dark" | "light"))
    {
        let joined = format!("{}{}", tokens[i], tokens[i + 1]);
        tokens.splice(i..i + 2, [joined]);
    }
    let mut kind = Variant::Default;
    let mut base = Vec::new();
    for token in tokens {
        if DARK.contains(&token.as_str()) {
            kind = Variant::Dark;
        } else if LIGHT.contains(&token.as_str()) {
            kind = Variant::Light;
        } else {
            base.push(token);
        }
    }
    (base.join("-"), kind)
}

fn extension_rank(path: &Path) -> u8 {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("svg") => 0,
        Some("png") => 1,
        Some("webp") => 2,
        _ => 3,
    }
}

/// The best light and dark logo files under `readme_dir`.
pub(super) fn find(fs: &dyn FileSystem, readme_dir: &Path) -> Option<(PathBuf, Option<PathBuf>)> {
    for folder in FOLDERS {
        let directory = if *folder == "." {
            readme_dir.to_path_buf()
        } else {
            readme_dir.join(folder)
        };
        if !fs.is_dir(&directory) {
            continue;
        }
        let mut groups: BTreeMap<String, BTreeMap<Variant, Vec<PathBuf>>> = BTreeMap::new();
        for path in fs.list_dir(&directory) {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let Some(captures) = LOGO_FILE.captures(name) else {
                continue;
            };
            if !fs.is_file(&path) {
                continue;
            }
            let (base, kind) = variant(&captures["stem"]);
            groups
                .entry(base)
                .or_default()
                .entry(kind)
                .or_default()
                .push(path);
        }
        let mut ranked: Vec<_> = groups.into_iter().collect();
        ranked.sort_by_key(|(base, _)| (base != "logo", base.len(), base.clone()));
        for (_, variants) in ranked {
            let best = |kind| {
                variants
                    .get(&kind)
                    .and_then(|files| files.iter().min_by_key(|p| extension_rank(p)))
                    .cloned()
            };
            if let Some(light) = best(Variant::Light).or_else(|| best(Variant::Default)) {
                return Some((light, best(Variant::Dark)));
            }
        }
    }
    None
}

/// The dark sibling of an explicitly configured logo, if there is one.
pub(super) fn dark_sibling(fs: &dyn FileSystem, logo: &Path) -> Option<PathBuf> {
    let (base, _) = variant(logo.file_stem()?.to_str()?);
    fs.list_dir(logo.parent()?)
        .into_iter()
        .filter(|p| p != logo)
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| IMAGE_FILE.captures(n))
                .is_some_and(|c| variant(&c["stem"]) == (base.clone(), Variant::Dark))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::MemoryFs;

    #[test]
    fn variants() {
        assert_eq!(variant("logo-dark"), ("logo".into(), Variant::Dark));
        assert_eq!(variant("Acme_Logo_White"), ("acme-logo".into(), Variant::Dark));
        assert_eq!(variant("logo-for-light"), ("logo".into(), Variant::Light));
        assert_eq!(variant("logo"), ("logo".into(), Variant::Default));
    }

    #[test]
    fn prefers_a_light_and_dark_pair_in_the_first_folder() {
        let fs = MemoryFs::new()
            .with("/p/assets/logo.svg", "")
            .with("/p/assets/logo-dark.svg", "")
            .with("/p/assets/logo.png", "")
            .with("/p/docs/logo-white.png", "");
        let (light, dark) = find(&fs, Path::new("/p")).expect("a logo");
        assert_eq!(light, PathBuf::from("/p/assets/logo.svg"));
        assert_eq!(dark, Some(PathBuf::from("/p/assets/logo-dark.svg")));
    }

    #[test]
    fn a_lone_dark_logo_is_not_used() {
        let fs = MemoryFs::new().with("/p/logo-dark.svg", "");
        assert!(find(&fs, Path::new("/p")).is_none());
    }
}
