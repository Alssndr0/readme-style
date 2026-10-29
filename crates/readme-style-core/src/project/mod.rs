//! Facts about the project around a README: metadata, CI, license, tools, logos.
//!
//! [`ProjectFacts`] is plain data. [`inspect`] gathers it from the files next to a README,
//! through a [`FileSystem`], without touching the network. Every fact records where it came
//! from, so a user can see why a badge was or was not added.

mod fs;
mod git;
mod inspect;
mod license;
mod logo;
mod workflows;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::Serialize;

pub use fs::{FileSystem, MemoryFs, OsFs};
pub use inspect::{Overrides, find_root, inspect};
pub use license::detect_license;
pub use workflows::parse_triggers;

/// A package registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Registry {
    Pypi,
    Npm,
    Crates,
    Go,
}

impl fmt::Display for Registry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Registry::Pypi => "pypi",
            Registry::Npm => "npm",
            Registry::Crates => "crates",
            Registry::Go => "go",
        })
    }
}

/// A package the project ships, and why we believe it is published, if we do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Package {
    pub registry: Registry,
    pub name: String,
    pub publish_evidence: Option<String>,
}

impl Package {
    pub fn new(registry: Registry, name: impl Into<String>) -> Self {
        Self {
            registry,
            name: name.into(),
            publish_evidence: None,
        }
    }
}

/// A GitHub Actions workflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Workflow {
    /// File name inside `.github/workflows`.
    pub file: String,
    /// The workflow's `name:`, or the file stem.
    pub name: String,
    /// Events from its `on:` key.
    pub triggers: BTreeSet<String>,
    #[serde(skip)]
    pub text: String,
}

/// Development tools whose use earns a badge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tool {
    PreCommit,
    Ruff,
    Black,
    Uv,
    Codecov,
    ReadTheDocs,
}

/// A logo, as paths relative to the README.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Logo {
    /// Shown in light mode, or always when there is no dark variant.
    pub light: String,
    /// Shown in dark mode.
    pub dark: Option<String>,
}

/// Everything readme-style knows about a project.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ProjectFacts {
    pub name: Option<String>,
    pub description: Option<String>,
    /// SPDX identifier (or expression).
    pub license: Option<String>,
    /// The license file, relative to the README.
    pub license_file: Option<String>,
    /// `owner/name` on GitHub.
    pub repo: Option<String>,
    pub repo_source: Option<String>,
    pub packages: Vec<Package>,
    pub python_requires: Option<String>,
    pub python_classifiers: Vec<String>,
    pub node_engine: Option<String>,
    pub rust_version: Option<String>,
    /// The Rust package has a library target (so docs.rs has something to show).
    pub rust_library: bool,
    pub go_version: Option<String>,
    pub workflows: Vec<Workflow>,
    /// Tool to evidence.
    pub tools: BTreeMap<Tool, String>,
    /// Lowercased labels (homepage, documentation, repository, ...) to URLs.
    pub urls: BTreeMap<String, String>,
    pub logo: Option<Logo>,
}

impl ProjectFacts {
    /// The documentation URL, unless it is just the repository page.
    pub fn docs_url(&self) -> Option<&str> {
        let url = self.urls.get("documentation").or_else(|| self.urls.get("docs"))?;
        if self.repo.is_some() && crate::urls::github_repo(url) == self.repo {
            return None;
        }
        Some(url)
    }
}
