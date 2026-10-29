//! Configuration: which theme to use, project facts to state, and theme overrides.
//!
//! Looked up from the README's folder up to the repository root, first match wins:
//! `.readme-style.toml`, `[tool.readme-style]` in `pyproject.toml`,
//! `[package.metadata.readme-style]` (or `[workspace.metadata.readme-style]`) in
//! `Cargo.toml`, and the `"readme-style"` key of `package.json`.
//!
//! ```toml
//! theme = "startup"
//! repo = "owner/name"
//!
//! [badges]
//! exclude = ["downloads"]
//!
//! [themes.brand]
//! extends = "startup"
//! badges.color = "ff6600"
//! ```

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use toml::{Table, Value};

use crate::project::{FileSystem, Overrides, find_root};
use crate::theme::{DEFAULT_THEME, Theme, ThemeError, Themes};

static REPO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[\w.-]+/[\w.-]+$").expect("valid regex"));

/// A configuration problem, with the file it came from.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{file}: {message}")]
pub struct ConfigError {
    /// The file (and table) the problem is in.
    pub file: String,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    /// Where the configuration was read from, if anywhere.
    pub source: Option<String>,
    pub theme: Option<String>,
    /// `owner/name` on GitHub, for repositories without a GitHub remote.
    pub repo: Option<String>,
    /// Theme tables merged over the selected theme.
    pub overrides: Table,
    /// Built-in themes plus the ones the configuration defines.
    pub themes: Themes,
}

impl Config {
    /// Read a configuration table. `source` names it in error messages.
    pub fn parse(mut table: Table, source: &str) -> Result<Self, ConfigError> {
        let error = |message: String| ConfigError {
            file: source.to_owned(),
            message,
        };
        let mut config = Config {
            source: Some(source.to_owned()),
            ..Config::default()
        };
        if let Some(themes) = table.remove("themes") {
            let Value::Table(themes) = themes else {
                return Err(error("'themes' must be a table of themes".into()));
            };
            for (name, definition) in themes {
                let Value::Table(definition) = definition else {
                    return Err(error(format!("themes.{name} must be a table")));
                };
                config
                    .themes
                    .define(&name, definition)
                    .map_err(|e| error(e.to_string()))?;
            }
        }
        if let Some(theme) = table.remove("theme") {
            let name = theme
                .as_str()
                .ok_or_else(|| error("'theme' must be a string".into()))?;
            if !config.themes.contains(name) {
                return Err(error(format!(
                    "unknown theme '{name}' (available: {})",
                    config.themes.names().join(", ")
                )));
            }
            config.theme = Some(name.to_owned());
        }
        if let Some(repo) = table.remove("repo") {
            let repo = repo
                .as_str()
                .filter(|r| REPO.is_match(r))
                .ok_or_else(|| error("'repo' must look like 'owner/name'".into()))?;
            config.repo = Some(repo.to_owned());
        }
        config.overrides = table;
        // Check the overrides now, so a typo is reported where it was made.
        config
            .themes
            .resolve(
                config.theme.as_deref().unwrap_or(DEFAULT_THEME),
                &config.overrides,
            )
            .map_err(|e| match e {
                ThemeError::Invalid { message, .. } => error(message),
                other => error(other.to_string()),
            })?;
        Ok(config)
    }

    /// The named theme with this configuration's overrides.
    pub fn theme(&self, name: &str) -> Result<Theme, ConfigError> {
        self.themes
            .resolve(name, &self.overrides)
            .map_err(|e| ConfigError {
                file: self.source.clone().unwrap_or_else(|| "theme".into()),
                message: e.to_string(),
            })
    }

    /// Facts the configuration states about the project.
    pub fn project_overrides(&self) -> Overrides {
        let logo = self
            .overrides
            .get("logo")
            .and_then(|l| l.get("path"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        Overrides {
            repo: self.repo.clone(),
            logo,
        }
    }
}

/// Find and read the configuration that applies to `readme`. No configuration is not an error.
pub fn discover(fs: &dyn FileSystem, readme: &Path) -> Result<Config, ConfigError> {
    let start = readme.parent().map_or_else(PathBuf::new, Path::to_path_buf);
    let root = find_root(fs, &start);
    for dir in start.ancestors() {
        if let Some((table, source)) = read_in(fs, dir)? {
            return Config::parse(table, &source);
        }
        if dir == root {
            break;
        }
    }
    Ok(Config::default())
}

fn read_in(fs: &dyn FileSystem, dir: &Path) -> Result<Option<(Table, String)>, ConfigError> {
    let parse_toml = |path: &Path| -> Result<Option<Table>, ConfigError> {
        let Some(text) = fs.read_to_string(path) else {
            return Ok(None);
        };
        text.parse::<Table>().map(Some).map_err(|e| ConfigError {
            file: path.display().to_string(),
            message: e.message().to_owned(),
        })
    };
    let dotfile = dir.join(".readme-style.toml");
    if let Some(table) = parse_toml(&dotfile)? {
        return Ok(Some((table, dotfile.display().to_string())));
    }
    let pyproject = dir.join("pyproject.toml");
    if let Some(table) = parse_toml(&pyproject)?.and_then(|t| nested(t, &["tool", "readme-style"])) {
        return Ok(Some((
            table,
            format!("{} [tool.readme-style]", pyproject.display()),
        )));
    }
    let cargo = dir.join("Cargo.toml");
    if let Some(manifest) = parse_toml(&cargo)? {
        for (section, keys) in [
            ("package", ["package", "metadata", "readme-style"]),
            ("workspace", ["workspace", "metadata", "readme-style"]),
        ] {
            if let Some(table) = nested(manifest.clone(), &keys) {
                return Ok(Some((
                    table,
                    format!("{} [{section}.metadata.readme-style]", cargo.display()),
                )));
            }
        }
    }
    let package = dir.join("package.json");
    if let Some(text) = fs.read_to_string(&package) {
        let source = format!("{} \"readme-style\"", package.display());
        let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| ConfigError {
            file: package.display().to_string(),
            message: e.to_string(),
        })?;
        if let Some(value) = json.get("readme-style") {
            let table = Table::try_from(value.clone()).map_err(|e| ConfigError {
                file: source.clone(),
                message: e.to_string(),
            })?;
            return Ok(Some((table, source)));
        }
    }
    Ok(None)
}

fn nested(table: Table, keys: &[&str]) -> Option<Table> {
    let mut current = Value::Table(table);
    for key in keys {
        current = current.as_table()?.get(*key)?.clone();
    }
    match current {
        Value::Table(table) => Some(table),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::badges::BadgeStyle;
    use crate::project::MemoryFs;

    fn readme() -> &'static Path {
        Path::new("/p/README.md")
    }

    #[test]
    fn no_configuration_is_fine() {
        let config = discover(&MemoryFs::new().with("/p/.git/config", ""), readme()).expect("ok");
        assert!(config.source.is_none() && config.theme.is_none());
    }

    #[test]
    fn reads_every_home() {
        let homes = [
            ("/p/.readme-style.toml", "theme = \"cli\"\n"),
            ("/p/pyproject.toml", "[tool.readme-style]\ntheme = \"cli\"\n"),
            (
                "/p/Cargo.toml",
                "[package]\nname = \"x\"\n[package.metadata.readme-style]\ntheme = \"cli\"\n",
            ),
            (
                "/p/package.json",
                "{\"name\": \"x\", \"readme-style\": {\"theme\": \"cli\"}}",
            ),
        ];
        for (path, content) in homes {
            let config = discover(&MemoryFs::new().with(path, content), readme()).expect(path);
            assert_eq!(config.theme.as_deref(), Some("cli"), "{path}");
        }
    }

    #[test]
    fn overrides_custom_themes_and_facts() {
        let text = "theme = \"brand\"\nrepo = \"me/x\"\n[logo]\npath = \"art/mark.svg\"\n\
                    [themes.brand]\nextends = \"startup\"\ndescription = \"Ours\"\nbadges.style = \"flat\"\n";
        let config = Config::parse(text.parse().expect("toml"), "test").expect("valid");
        let theme = config.theme("brand").expect("resolves");
        assert_eq!(theme.badges.style, BadgeStyle::Flat);
        let overrides = config.project_overrides();
        assert_eq!(
            (overrides.repo.as_deref(), overrides.logo.as_deref()),
            (Some("me/x"), Some("art/mark.svg"))
        );
    }

    #[test]
    fn errors_name_the_file_and_the_problem() {
        let parse = |text: &str| {
            Config::parse(text.parse().expect("toml"), "cfg.toml")
                .expect_err("invalid")
                .to_string()
        };
        assert_eq!(
            parse("theme = \"neon\""),
            "cfg.toml: unknown theme 'neon' (available: minimal, startup, ml-research, cli, docs)"
        );
        assert_eq!(
            parse("repo = \"nope\""),
            "cfg.toml: 'repo' must look like 'owner/name'"
        );
        assert!(parse("colour = \"red\"").starts_with("cfg.toml: unknown field `colour`"));
    }
}
