//! Every badge readme-style knows, and the groups themes select them by.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

macro_rules! ids {
    ($(#[$meta:meta])* $vis:vis enum $name:ident { $($variant:ident => $id:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $vis enum $name { $($variant),+ }

        impl $name {
            /// Every value, in catalog order.
            pub const ALL: &[$name] = &[$($name::$variant),+];

            /// The identifier used in themes and configuration.
            pub fn id(self) -> &'static str {
                match self { $($name::$variant => $id),+ }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.id())
            }
        }

        impl Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.id())
            }
        }

        impl FromStr for $name {
            type Err = ();
            fn from_str(s: &str) -> Result<Self, ()> {
                Self::ALL.iter().copied().find(|v| v.id() == s).ok_or(())
            }
        }
    };
}

ids! {
    /// A kind of badge.
    pub enum BadgeKind {
        Pypi => "pypi",
        Npm => "npm",
        Crates => "crates",
        Go => "go",
        PypiDownloads => "pypi-downloads",
        NpmDownloads => "npm-downloads",
        CratesDownloads => "crates-downloads",
        Python => "python",
        Node => "node",
        Rust => "rust",
        GoVersion => "go-version",
        Ci => "ci",
        Codecov => "codecov",
        Docs => "docs",
        Docsrs => "docsrs",
        License => "license",
        Ruff => "ruff",
        Black => "black",
        Uv => "uv",
        PreCommit => "pre-commit",
        Arxiv => "arxiv",
        Paper => "paper",
        ProjectPage => "project-page",
        HfModels => "hf-models",
        HfDatasets => "hf-datasets",
        HfSpaces => "hf-spaces",
        Colab => "colab",
        Discord => "discord",
    }
}

ids! {
    /// A family of badges a theme can ask for at once.
    pub enum BadgeGroup {
        Package => "package",
        Downloads => "downloads",
        Runtime => "runtime",
        Ci => "ci",
        Coverage => "coverage",
        Docs => "docs",
        License => "license",
        Tooling => "tooling",
        Research => "research",
        Community => "community",
    }
}

impl BadgeGroup {
    /// The badges in this group, in display order.
    pub fn kinds(self) -> &'static [BadgeKind] {
        use BadgeKind as K;
        match self {
            BadgeGroup::Package => &[K::Pypi, K::Npm, K::Crates, K::Go],
            BadgeGroup::Downloads => &[K::PypiDownloads, K::NpmDownloads, K::CratesDownloads],
            BadgeGroup::Runtime => &[K::Python, K::Node, K::Rust, K::GoVersion],
            BadgeGroup::Ci => &[K::Ci],
            BadgeGroup::Coverage => &[K::Codecov],
            BadgeGroup::Docs => &[K::Docs, K::Docsrs],
            BadgeGroup::License => &[K::License],
            BadgeGroup::Tooling => &[K::Ruff, K::Black, K::Uv, K::PreCommit],
            BadgeGroup::Research => &[
                K::Arxiv,
                K::Paper,
                K::ProjectPage,
                K::HfModels,
                K::HfDatasets,
                K::HfSpaces,
                K::Colab,
            ],
            BadgeGroup::Community => &[K::Discord],
        }
    }
}

/// A group or a single badge, as written in a theme or configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum BadgeSelector {
    Group(BadgeGroup),
    Kind(BadgeKind),
}

impl BadgeSelector {
    pub fn kinds(self) -> Vec<BadgeKind> {
        match self {
            BadgeSelector::Group(group) => group.kinds().to_vec(),
            BadgeSelector::Kind(kind) => vec![kind],
        }
    }
}

impl TryFrom<String> for BadgeSelector {
    type Error = String;

    /// Groups win over badges of the same name (`ci`, `docs` and `license` are both).
    fn try_from(value: String) -> Result<Self, String> {
        if let Ok(group) = value.parse() {
            return Ok(BadgeSelector::Group(group));
        }
        if let Ok(kind) = value.parse() {
            return Ok(BadgeSelector::Kind(kind));
        }
        let groups: Vec<_> = BadgeGroup::ALL.iter().map(|g| g.id()).collect();
        Err(format!(
            "unknown badge or group '{value}' (groups: {})",
            groups.join(", ")
        ))
    }
}

impl From<BadgeSelector> for String {
    fn from(selector: BadgeSelector) -> String {
        match selector {
            BadgeSelector::Group(group) => group.id().to_owned(),
            BadgeSelector::Kind(kind) => kind.id().to_owned(),
        }
    }
}

/// Expand selectors to badge kinds, keeping the first occurrence of each.
pub fn expand(selectors: &[BadgeSelector]) -> Vec<BadgeKind> {
    let mut kinds = Vec::new();
    for kind in selectors.iter().flat_map(|s| s.kinds()) {
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }
    kinds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_badge_is_in_exactly_one_group() {
        for kind in BadgeKind::ALL {
            let groups = BadgeGroup::ALL
                .iter()
                .filter(|g| g.kinds().contains(kind))
                .count();
            assert_eq!(groups, 1, "{kind}");
        }
    }

    #[test]
    fn selectors_parse_groups_first() {
        assert_eq!(
            BadgeSelector::try_from("ci".to_owned()),
            Ok(BadgeSelector::Group(BadgeGroup::Ci))
        );
        assert_eq!(
            BadgeSelector::try_from("ruff".to_owned()),
            Ok(BadgeSelector::Kind(BadgeKind::Ruff))
        );
        assert!(BadgeSelector::try_from("neon".to_owned()).is_err());
        let kinds = expand(&[
            BadgeSelector::Kind(BadgeKind::License),
            BadgeSelector::Group(BadgeGroup::License),
        ]);
        assert_eq!(kinds, [BadgeKind::License]);
    }
}
