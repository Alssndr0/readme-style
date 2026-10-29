//! Gathering [`ProjectFacts`] from the files around a README.

use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value as Json;
use toml::{Table, Value};

use super::{FileSystem, Logo, Package, ProjectFacts, Registry, Tool, git, license, logo, workflows};
use crate::urls;

/// Facts the user states in configuration, which win over inference.
#[derive(Debug, Clone, Default)]
pub struct Overrides {
    /// `owner/name` on GitHub.
    pub repo: Option<String>,
    /// Logo path relative to the README.
    pub logo: Option<String>,
}

static PYTHON_CLASSIFIER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Programming Language :: Python :: (3\.\d+)$").expect("valid regex"));
static GO_MODULE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^module\s+(\S+)").expect("valid regex"));
static GO_VERSION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^go\s+(\d+\.\d+)").expect("valid regex"));

const LICENSE_FILES: &[&str] = &[
    "LICENSE",
    "LICENSE.md",
    "LICENSE.txt",
    "LICENCE",
    "LICENCE.md",
    "COPYING",
    "LICENSE-MIT",
    "LICENSE-APACHE",
];

struct Inspector<'a> {
    fs: &'a dyn FileSystem,
    readme_dir: PathBuf,
    root: PathBuf,
    facts: ProjectFacts,
}

/// Gather what can be known offline about the project that `readme` belongs to.
///
/// Manifests and license files are searched from the README's folder up to the repository
/// root, the nearest folder holding `.git` or `.github`.
pub fn inspect(fs: &dyn FileSystem, readme: &Path, overrides: &Overrides) -> ProjectFacts {
    let readme_dir = readme.parent().map_or_else(PathBuf::new, Path::to_path_buf);
    let root = find_root(fs, &readme_dir);
    let mut inspector = Inspector {
        fs,
        readme_dir,
        root,
        facts: ProjectFacts::default(),
    };
    inspector.python();
    inspector.node();
    inspector.rust();
    inspector.go();
    inspector.license();
    inspector.workflows();
    inspector.tools();
    inspector.repo(overrides.repo.as_deref());
    inspector.logo(overrides.logo.as_deref());
    let mut facts = inspector.facts;
    facts.description = facts
        .description
        .map(|d| d.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|d| !d.is_empty());
    facts
}

/// The nearest folder at or above `start` that looks like a repository root.
pub fn find_root(fs: &dyn FileSystem, start: &Path) -> PathBuf {
    start
        .ancestors()
        .find(|dir| fs.exists(&dir.join(".git")) || fs.is_dir(&dir.join(".github")))
        .unwrap_or(start)
        .to_path_buf()
}

/// `target` relative to `base`, with forward slashes.
fn relative(base: &Path, target: &Path) -> String {
    let base: Vec<Component<'_>> = base.components().collect();
    let target: Vec<Component<'_>> = target.components().collect();
    let common = base.iter().zip(&target).take_while(|(a, b)| a == b).count();
    let mut parts: Vec<String> = vec!["..".to_owned(); base.len() - common];
    parts.extend(
        target[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    parts.join("/")
}

fn string(table: &Table, key: &str) -> Option<String> {
    table.get(key).and_then(Value::as_str).map(str::to_owned)
}

impl Inspector<'_> {
    /// The first of `names` found from the README folder up to the root. Names are matched
    /// against directory listings, so the result (and the real file name) is the same on every
    /// file system; `any_case` also accepts `License` for `LICENSE`.
    fn find(&self, names: &[&str], any_case: bool) -> Option<PathBuf> {
        for dir in self.readme_dir.ancestors() {
            let entries = self.fs.list_dir(dir);
            for name in names {
                let found = entries.iter().find(|path| {
                    path.file_name().and_then(|n| n.to_str()).is_some_and(|actual| {
                        if any_case {
                            actual.eq_ignore_ascii_case(name)
                        } else {
                            actual == *name
                        }
                    }) && self.fs.is_file(path)
                });
                if let Some(path) = found {
                    return Some(path.clone());
                }
            }
            if dir == self.root {
                break;
            }
        }
        None
    }

    fn search(&self, names: &[&str]) -> Option<PathBuf> {
        self.find(names, false)
    }

    fn toml(&self, name: &str) -> Option<Table> {
        self.fs
            .read_to_string(&self.search(&[name])?)?
            .parse::<Table>()
            .ok()
    }

    fn rel(&self, path: &Path) -> String {
        relative(&self.readme_dir, path)
    }

    fn adopt(&mut self, name: Option<String>, description: Option<String>) {
        if self.facts.name.is_none() {
            self.facts.name = name;
        }
        if self.facts.description.is_none() {
            self.facts.description = description;
        }
    }

    fn adopt_license(&mut self, value: Option<&str>) {
        if self.facts.license.is_none() {
            self.facts.license = value.and_then(license::spdx);
        }
    }

    fn url(&mut self, label: &str, url: Option<String>) {
        if let Some(url) = url {
            self.facts.urls.entry(label.to_ascii_lowercase()).or_insert(url);
        }
    }

    fn python(&mut self) {
        let Some(data) = self.toml("pyproject.toml") else {
            return;
        };
        let poetry = data
            .get("tool")
            .and_then(|t| t.get("poetry"))
            .and_then(Value::as_table);
        let Some(meta) = data.get("project").and_then(Value::as_table).or(poetry) else {
            return;
        };
        let Some(name) = string(meta, "name") else { return };
        self.adopt(Some(name.clone()), string(meta, "description"));
        self.facts.packages.push(Package::new(Registry::Pypi, name));
        self.facts.python_requires = string(meta, "requires-python").or_else(|| {
            meta.get("dependencies")
                .and_then(Value::as_table)
                .and_then(|d| string(d, "python"))
        });
        let classifiers: Vec<&str> = meta
            .get("classifiers")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        self.facts.python_classifiers = classifiers
            .iter()
            .filter_map(|c| PYTHON_CLASSIFIER.captures(c))
            .map(|c| c[1].to_owned())
            .collect();
        match meta.get("license") {
            Some(Value::String(text)) => self.adopt_license(Some(text)),
            Some(Value::Table(table)) => self.adopt_license(table.get("text").and_then(Value::as_str)),
            _ => {}
        }
        if self.facts.license.is_none() {
            let from_classifier = classifiers
                .iter()
                .filter_map(|c| c.strip_prefix("License :: OSI Approved :: "))
                .find_map(license::from_classifier);
            self.facts.license = from_classifier.map(str::to_owned);
        }
        for (label, url) in meta.get("urls").and_then(Value::as_table).into_iter().flatten() {
            self.url(label, url.as_str().map(str::to_owned));
        }
        self.url("homepage", string(meta, "homepage"));
        self.url("repository", string(meta, "repository"));
        self.url("documentation", string(meta, "documentation"));
    }

    fn node(&mut self) {
        let Some(path) = self.search(&["package.json"]) else {
            return;
        };
        let Some(data) = self
            .fs
            .read_to_string(&path)
            .and_then(|t| serde_json::from_str::<Json>(&t).ok())
        else {
            return;
        };
        let text = |key: &str| data.get(key).and_then(Json::as_str).map(str::to_owned);
        let Some(name) = text("name") else { return };
        self.adopt(Some(name.clone()), text("description"));
        if !data.get("private").and_then(Json::as_bool).unwrap_or(false) {
            self.facts.packages.push(Package::new(Registry::Npm, name));
        }
        self.facts.node_engine = data
            .pointer("/engines/node")
            .and_then(Json::as_str)
            .map(str::to_owned);
        self.adopt_license(data.get("license").and_then(Json::as_str));
        self.url("homepage", text("homepage"));
        let repository = data
            .get("repository")
            .and_then(|r| r.as_str().or_else(|| r.get("url")?.as_str()));
        self.url("repository", repository.map(str::to_owned));
    }

    fn rust(&mut self) {
        let Some(data) = self.toml("Cargo.toml") else {
            return;
        };
        let Some(package) = data.get("package").and_then(Value::as_table) else {
            return;
        };
        let Some(name) = string(package, "name") else {
            return;
        };
        // `key.workspace = true` takes the value from the workspace root manifest.
        let inherited = data
            .get("workspace")
            .and_then(|w| w.get("package"))
            .and_then(Value::as_table);
        let field = |key: &str| match package.get(key) {
            Some(Value::String(value)) => Some(value.clone()),
            Some(Value::Table(t)) if t.get("workspace").and_then(Value::as_bool) == Some(true) => {
                inherited.and_then(|w| string(w, key))
            }
            _ => None,
        };
        self.adopt(Some(name.clone()), field("description"));
        if package.get("publish").and_then(Value::as_bool) != Some(false) {
            self.facts.packages.push(Package::new(Registry::Crates, name));
        }
        self.facts.rust_version = field("rust-version");
        self.adopt_license(field("license").as_deref());
        let manifest_dir = self
            .search(&["Cargo.toml"])
            .and_then(|p| p.parent().map(Path::to_path_buf));
        self.facts.rust_library = data.contains_key("lib")
            || manifest_dir.is_some_and(|dir| self.fs.is_file(&dir.join("src").join("lib.rs")));
        for key in ["homepage", "documentation", "repository"] {
            self.url(key, field(key));
        }
    }

    fn go(&mut self) {
        let Some(text) = self.search(&["go.mod"]).and_then(|p| self.fs.read_to_string(&p)) else {
            return;
        };
        let Some(module) = GO_MODULE.captures(&text).map(|c| c[1].to_owned()) else {
            return;
        };
        let short = module.rsplit('/').next().map(str::to_owned);
        self.adopt(short, None);
        self.facts.packages.push(Package::new(Registry::Go, module));
        self.facts.go_version = GO_VERSION.captures(&text).map(|c| c[1].to_owned());
    }

    fn license(&mut self) {
        let Some(path) = self.find(LICENSE_FILES, true) else {
            return;
        };
        self.facts.license_file = Some(self.rel(&path));
        if self.facts.license.as_deref().is_none_or(|l| l == "BSD")
            && let Some(found) = self
                .fs
                .read_to_string(&path)
                .as_deref()
                .and_then(license::detect_license)
        {
            self.facts.license = Some(found.to_owned());
        }
    }

    fn workflows(&mut self) {
        self.facts.workflows = workflows::read(self.fs, &self.root);
        for package in &mut self.facts.packages {
            package.publish_evidence = workflows::publish_evidence(&self.facts.workflows, package.registry);
        }
    }

    fn tools(&mut self) {
        let pyproject = self.toml("pyproject.toml").unwrap_or_default();
        let tool = pyproject
            .get("tool")
            .and_then(Value::as_table)
            .cloned()
            .unwrap_or_default();
        let precommit = self
            .fs
            .read_to_string(&self.root.join(".pre-commit-config.yaml"))
            .unwrap_or_default();
        let mut found: Vec<(Tool, String)> = Vec::new();
        if !precommit.is_empty() {
            found.push((Tool::PreCommit, ".pre-commit-config.yaml".into()));
        }
        if tool.contains_key("ruff") {
            found.push((Tool::Ruff, "[tool.ruff] in pyproject.toml".into()));
        } else if let Some(path) = self.search(&["ruff.toml", ".ruff.toml"]) {
            found.push((Tool::Ruff, file_name(&path)));
        } else if precommit.contains("astral-sh/ruff-pre-commit") {
            found.push((Tool::Ruff, "ruff hook in .pre-commit-config.yaml".into()));
        }
        if tool.contains_key("black") {
            found.push((Tool::Black, "[tool.black] in pyproject.toml".into()));
        } else if precommit.contains("psf/black") {
            found.push((Tool::Black, "black hook in .pre-commit-config.yaml".into()));
        }
        if let Some(path) = self.search(&["uv.lock"]) {
            found.push((Tool::Uv, file_name(&path)));
        }
        if let Some(path) = self.search(&["codecov.yml", ".codecov.yml"]) {
            found.push((Tool::Codecov, file_name(&path)));
        } else if let Some(w) = self
            .facts
            .workflows
            .iter()
            .find(|w| w.text.contains("codecov/codecov-action"))
        {
            found.push((Tool::Codecov, format!("codecov action in {}", w.file)));
        }
        if let Some(path) = self.search(&[".readthedocs.yaml", ".readthedocs.yml", "readthedocs.yml"]) {
            found.push((Tool::ReadTheDocs, file_name(&path)));
        }
        self.facts.tools.extend(found);
    }

    fn repo(&mut self, configured: Option<&str>) {
        if let Some(repo) = configured {
            self.facts.repo = Some(repo.to_owned());
            self.facts.repo_source = Some("config".into());
            return;
        }
        if let Some(repo) = git::origin_url(self.fs, &self.root)
            .as_deref()
            .and_then(urls::github_repo)
        {
            self.facts.repo = Some(repo);
            self.facts.repo_source = Some("git remote origin".into());
            return;
        }
        for label in ["repository", "source", "source code", "code", "homepage"] {
            if let Some(repo) = self.facts.urls.get(label).and_then(|u| urls::github_repo(u)) {
                self.facts.repo = Some(repo);
                self.facts.repo_source = Some(format!("'{label}' URL in project metadata"));
                return;
            }
        }
    }

    fn logo(&mut self, configured: Option<&str>) {
        let found = match configured {
            Some(path) => {
                let path = self.readme_dir.join(path);
                self.fs.is_file(&path).then(|| {
                    let dark = logo::dark_sibling(self.fs, &path);
                    (path, dark)
                })
            }
            None => logo::find(self.fs, &self.readme_dir),
        };
        self.facts.logo = found.map(|(light, dark)| Logo {
            light: urls::normalize_relative(&self.rel(&light)),
            dark: dark.map(|d| urls::normalize_relative(&self.rel(&d))),
        });
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::MemoryFs;

    const PYPROJECT: &str = r#"
[project]
name = "quickparse"
description = """A tiny
  parser."""
requires-python = ">=3.10"
license = "MIT"

[project.urls]
Documentation = "https://quickparse.readthedocs.io"
Repository = "https://github.com/acme/quickparse"

[tool.ruff]
line-length = 100
"#;

    fn readme() -> &'static Path {
        Path::new("/p/README.md")
    }

    #[test]
    fn python_project() {
        let fs = MemoryFs::new()
            .with("/p/.github/workflows/release.yml", "on:\n  release:\njobs:\n  x:\n    steps:\n      - uses: pypa/gh-action-pypi-publish@release/v1\n")
            .with("/p/pyproject.toml", PYPROJECT)
            .with("/p/uv.lock", "");
        let facts = inspect(&fs, readme(), &Overrides::default());
        assert_eq!(facts.name.as_deref(), Some("quickparse"));
        assert_eq!(facts.description.as_deref(), Some("A tiny parser."));
        assert_eq!(facts.license.as_deref(), Some("MIT"));
        assert_eq!(facts.repo.as_deref(), Some("acme/quickparse"));
        assert_eq!(
            facts.packages[0].publish_evidence.as_deref(),
            Some("release.yml runs pypa/gh-action-pypi-publish")
        );
        assert_eq!(
            facts.tools.keys().copied().collect::<Vec<_>>(),
            [Tool::Ruff, Tool::Uv]
        );
        assert_eq!(facts.docs_url(), Some("https://quickparse.readthedocs.io"));
    }

    #[test]
    fn node_project_and_private_packages() {
        let json = r#"{"name": "tidy-cli", "license": "ISC", "engines": {"node": ">=18"},
                       "repository": {"url": "git+https://github.com/acme/tidy-cli.git"}}"#;
        let facts = inspect(
            &MemoryFs::new().with("/p/package.json", json),
            readme(),
            &Overrides::default(),
        );
        assert_eq!(facts.packages, [Package::new(Registry::Npm, "tidy-cli")]);
        assert_eq!(facts.node_engine.as_deref(), Some(">=18"));
        assert_eq!(facts.repo.as_deref(), Some("acme/tidy-cli"));
        let private = r#"{"name": "app", "private": true}"#;
        let facts = inspect(
            &MemoryFs::new().with("/p/package.json", private),
            readme(),
            &Overrides::default(),
        );
        assert!(facts.packages.is_empty());
    }

    #[test]
    fn license_file_and_git_remote_above_a_subfolder_readme() {
        let fs = MemoryFs::new()
            .with(
                "/r/.git/config",
                "[remote \"origin\"]\n\turl = https://github.com/acme/mono.git\n",
            )
            .with("/r/LICENSE", "Apache License\nVersion 2.0, January 2004")
            .with(
                "/r/pkg/Cargo.toml",
                "[package]\nname = \"widget\"\nrust-version = \"1.80\"\n",
            )
            .with("/r/pkg/src/lib.rs", "");
        let facts = inspect(&fs, Path::new("/r/pkg/README.md"), &Overrides::default());
        assert_eq!(facts.license.as_deref(), Some("Apache-2.0"));
        assert_eq!(facts.license_file.as_deref(), Some("../LICENSE"));
        assert_eq!(facts.repo.as_deref(), Some("acme/mono"));
        assert_eq!(facts.rust_version.as_deref(), Some("1.80"));
        assert!(facts.rust_library);
    }

    #[test]
    fn workspace_inheritance() {
        let manifest = "[package]\nname = \"tool\"\nlicense.workspace = true\nrust-version.workspace = true\n\n\
                        [workspace.package]\nlicense = \"MIT\"\nrust-version = \"1.85\"\n";
        let facts = inspect(
            &MemoryFs::new().with("/p/Cargo.toml", manifest),
            readme(),
            &Overrides::default(),
        );
        assert_eq!(
            (facts.license.as_deref(), facts.rust_version.as_deref()),
            (Some("MIT"), Some("1.85"))
        );
        assert!(
            !facts.rust_library,
            "a binary-only crate has no docs.rs page worth a badge"
        );
    }

    #[test]
    fn license_files_match_any_case_and_keep_their_real_name() {
        let fs = MemoryFs::new().with(
            "/p/License",
            "MIT License\nPermission is hereby granted, free of charge",
        );
        let facts = inspect(&fs, readme(), &Overrides::default());
        assert_eq!(
            (facts.license.as_deref(), facts.license_file.as_deref()),
            (Some("MIT"), Some("License"))
        );
    }

    #[test]
    fn overrides_win() {
        let fs = MemoryFs::new()
            .with("/p/brand/mark.svg", "")
            .with("/p/brand/mark-dark.svg", "")
            .with("/p/logo.png", "");
        let overrides = Overrides {
            repo: Some("me/x".into()),
            logo: Some("./brand/mark.svg".into()),
        };
        let facts = inspect(&fs, readme(), &overrides);
        assert_eq!(facts.repo_source.as_deref(), Some("config"));
        let logo = facts.logo.expect("logo");
        assert_eq!(
            (logo.light.as_str(), logo.dark.as_deref()),
            ("brand/mark.svg", Some("brand/mark-dark.svg"))
        );
    }

    #[test]
    fn relative_paths() {
        assert_eq!(relative(Path::new("/a/b"), Path::new("/a/b/c/d.svg")), "c/d.svg");
        assert_eq!(relative(Path::new("/a/b"), Path::new("/a/LICENSE")), "../LICENSE");
    }
}
