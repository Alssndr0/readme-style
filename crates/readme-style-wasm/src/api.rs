//! Requests from the playground, answered by the same engine with the same rules as the command
//! line: configuration discovery, theme choice and project inspection all run, on files held in
//! memory instead of on disk.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use readme_style_core::badges::Badge;
use readme_style_core::config;
use readme_style_core::project::{MemoryFs, inspect};
use readme_style_core::theme::{DEFAULT_THEME, Themes};
use readme_style_core::{Diagnostic, recorded_theme, style};
use serde::{Deserialize, Serialize};
use similar::TextDiff;

/// Where the project sits in the in-memory file system.
const ROOT: &str = "/project";

/// What to style, and the project around it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StyleRequest {
    /// The README's text.
    pub readme: String,
    /// The README's path inside the project.
    #[serde(default = "default_readme_path")]
    pub path: String,
    /// The project's other files, by path inside the project. Content matters only for the files
    /// readme-style reads (manifests, workflows, license, configuration); for any other file,
    /// such as an image, an empty string says that it exists.
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    /// A theme; otherwise the configuration's, the one recorded in the README, or the default.
    #[serde(default)]
    pub theme: Option<String>,
    /// `owner/name` on GitHub, which wins over the configuration and the git remote.
    #[serde(default)]
    pub repo: Option<String>,
}

fn default_readme_path() -> String {
    "README.md".into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Changed,
    Unchanged,
    /// The guard refused: styling would have changed what the README says.
    Refused,
    /// The request could not be handled, such as an unknown theme or a broken configuration.
    Error,
}

/// The result, with everything the command line would report.
#[derive(Debug, Serialize)]
pub struct StyleResponse {
    pub status: Status,
    /// Why, for `refused` and `error`.
    pub message: Option<String>,
    pub theme: Option<String>,
    /// Where the theme choice came from.
    pub theme_source: Option<String>,
    pub output: Option<String>,
    /// A unified diff from the README to the output, when they differ.
    pub diff: Option<String>,
    pub changed_by: Vec<&'static str>,
    /// The badges in the header, each with the evidence for it.
    pub badges: Vec<Badge>,
    pub diagnostics: Vec<Diagnostic>,
    pub warnings: Vec<String>,
}

impl StyleResponse {
    fn new(status: Status) -> Self {
        Self {
            status,
            message: None,
            theme: None,
            theme_source: None,
            output: None,
            diff: None,
            changed_by: Vec::new(),
            badges: Vec::new(),
            diagnostics: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// A request that could not be handled.
    pub fn error(message: impl Into<String>, warnings: Vec<String>) -> Self {
        Self {
            message: Some(message.into()),
            warnings,
            ..Self::new(Status::Error)
        }
    }
}

/// `text` with in-memory paths shown as paths inside the project.
fn local(text: &str) -> String {
    text.replace(&format!("{ROOT}/"), "")
}

/// A path inside the project, or `None` for anything that could point outside it.
fn inside(relative: &str) -> Option<PathBuf> {
    let path = Path::new(relative);
    let plain = !relative.is_empty() && path.components().all(|c| matches!(c, Component::Normal(_)));
    plain.then(|| Path::new(ROOT).join(path))
}

/// Style a README the way `readme-style` would in a checkout holding `request.files`.
pub fn style_readme(request: &StyleRequest) -> StyleResponse {
    let mut warnings = Vec::new();
    let mut fs = MemoryFs::new();
    for (path, content) in &request.files {
        match inside(path) {
            Some(path) => {
                fs.insert(path, content.as_str());
            }
            None => warnings.push(format!("ignored {path}: not a path inside the project")),
        }
    }
    let Some(readme) = inside(&request.path) else {
        return StyleResponse::error(
            format!("{} is not a path inside the project", request.path),
            warnings,
        );
    };
    fs.insert(readme.clone(), request.readme.as_str());
    let config = match config::discover(&fs, &readme) {
        Ok(config) => config,
        Err(error) => return StyleResponse::error(local(&error.to_string()), warnings),
    };
    let (name, source) = if let Some(name) = request.theme.as_deref().filter(|n| !n.is_empty()) {
        (name.to_owned(), "chosen".to_owned())
    } else if let Some(name) = &config.theme {
        (
            name.clone(),
            config.source.clone().unwrap_or_else(|| "config".into()),
        )
    } else if let Some(name) = recorded_theme(&request.readme).filter(|n| config.themes.contains(n)) {
        (name, "markers in the README".to_owned())
    } else {
        (DEFAULT_THEME.to_owned(), "default".to_owned())
    };
    let theme = match config.theme(&name) {
        Ok(theme) => theme,
        Err(error) => return StyleResponse::error(local(&error.to_string()), warnings),
    };
    let mut overrides = config.project_overrides();
    if let Some(repo) = request.repo.as_deref().map(str::trim).filter(|r| !r.is_empty()) {
        overrides.repo = Some(repo.to_owned());
    }
    let facts = inspect(&fs, &readme, &overrides);
    let mut response = StyleResponse {
        theme: Some(theme.name.clone()),
        theme_source: Some(local(&source)),
        warnings,
        ..StyleResponse::new(Status::Unchanged)
    };
    match style(&request.readme, &theme, &facts) {
        Ok(outcome) => {
            if outcome.changed {
                response.status = Status::Changed;
                response.diff = Some(
                    TextDiff::from_lines(&request.readme, &outcome.output)
                        .unified_diff()
                        .context_radius(3)
                        .header(
                            &format!("{} (current)", request.path),
                            &format!("{} (styled)", request.path),
                        )
                        .to_string(),
                );
            }
            response.output = Some(outcome.output);
            response.changed_by = outcome.changed_by;
            response.badges = outcome.badges;
            response.diagnostics = outcome.diagnostics;
        }
        Err(refusal) => {
            response.status = Status::Refused;
            response.message = Some(refusal.to_string());
        }
    }
    response
}

/// A built-in theme, for the theme picker.
#[derive(Debug, Serialize)]
pub struct ThemeInfo {
    pub name: String,
    pub description: String,
}

pub fn themes() -> Vec<ThemeInfo> {
    let themes = Themes::builtin();
    themes
        .names()
        .into_iter()
        .filter_map(|name| themes.get(name).ok())
        .map(|theme| ThemeInfo {
            name: theme.name,
            description: theme.description,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Status, StyleRequest, inside, style_readme, themes};

    fn request(json: &str) -> StyleRequest {
        serde_json::from_str(json).expect("a valid request")
    }

    #[test]
    fn styles_with_badges_from_the_project_files() {
        let response = style_readme(&request(
            r##"{
                "readme": "# demo\n\n> **Note:** hello\n",
                "files": {
                    ".github/workflows/ci.yml": "name: CI\non: [push]\n",
                    "LICENSE": "MIT License\n\nPermission is hereby granted, free of charge",
                    ".readme-style.toml": "theme = \"startup\"\n"
                },
                "repo": "acme/demo"
            }"##,
        ));
        assert_eq!(response.status, Status::Changed);
        assert_eq!(response.theme.as_deref(), Some("startup"));
        assert_eq!(response.theme_source.as_deref(), Some(".readme-style.toml"));
        let output = response.output.expect("output");
        assert!(output.contains("> [!NOTE]"), "{output}");
        assert!(output.contains("acme/demo/actions/workflows/ci.yml"), "{output}");
        assert!(response.badges.iter().any(|b| b.reason.contains("LICENSE")));
        assert!(response.diff.expect("diff").contains("+> [!NOTE]"));
    }

    #[test]
    fn the_chosen_theme_wins_and_unknown_ones_are_errors() {
        let chosen = style_readme(&request(r##"{"readme": "# demo\n", "theme": "docs"}"##));
        assert_eq!(chosen.theme.as_deref(), Some("docs"));
        let unknown = style_readme(&request(r##"{"readme": "# demo\n", "theme": "neon"}"##));
        assert_eq!(unknown.status, Status::Error);
        assert!(unknown.message.expect("message").contains("neon"));
    }

    #[test]
    fn paths_stay_inside_the_project() {
        assert!(inside("docs/logo.svg").is_some());
        for path in ["", "/etc/passwd", "../outside", "a/../../b", "./README.md"] {
            assert!(inside(path).is_none(), "{path}");
        }
        let response = style_readme(&request(r##"{"readme": "# demo\n", "files": {"../x": ""}}"##));
        assert_eq!(response.warnings.len(), 1);
        let response = style_readme(&request(r##"{"readme": "# demo\n", "path": "../README.md"}"##));
        assert_eq!(response.status, Status::Error);
    }

    #[test]
    fn lists_the_built_in_themes() {
        let names: Vec<String> = themes().into_iter().map(|t| t.name).collect();
        assert!(names.contains(&"minimal".to_owned()) && names.contains(&"docs".to_owned()));
    }
}
