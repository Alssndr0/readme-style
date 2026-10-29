//! Themes: plain data saying which pieces to add and how to lay them out.
//!
//! GitHub strips custom CSS, so a theme's visual range comes from layout (centering through
//! `align`), badge style and colors, navigation, collapsible sections and callouts.
//!
//! Built-in themes are TOML files embedded in the library. Each one lists only what differs
//! from `base.toml` (or from the theme it `extends`). Custom themes and configuration
//! overrides are merged the same way, then the result is checked against [`Theme`], which
//! rejects unknown keys and bad values.

use std::collections::BTreeMap;

use regex::Regex;
use serde::{Deserialize, Serialize};
use toml::{Table, Value};

use crate::badges::{BadgeSelector, BadgeStyle, Look};

const BASE: &str = "base";
const BUILTIN: &[(&str, &str)] = &[
    (BASE, include_str!("../themes/base.toml")),
    ("minimal", include_str!("../themes/minimal.toml")),
    ("startup", include_str!("../themes/startup.toml")),
    ("ml-research", include_str!("../themes/ml-research.toml")),
    ("cli", include_str!("../themes/cli.toml")),
    ("docs", include_str!("../themes/docs.toml")),
];

/// The theme used when nothing else says which one.
pub const DEFAULT_THEME: &str = "minimal";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Align {
    Left,
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HeaderPosition {
    /// Badges and navigation right under the title.
    AfterTitle,
    /// Under the tagline and any badge rows the README already has.
    AfterTagline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NavStyle {
    Bold,
    Plain,
    /// Each link in a `<kbd>` key cap.
    Kbd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TocStyle {
    None,
    List,
    /// A list folded into `<details>`.
    Details,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Layout {
    pub align: Align,
    pub header_position: HeaderPosition,
    pub divider: bool,
    pub tagline_from_metadata: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct LogoSettings {
    pub show: bool,
    pub width: u32,
    pub dark_variants: bool,
    /// A logo path relative to the README, set in configuration rather than in themes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct BadgeSettings {
    pub include: Vec<BadgeSelector>,
    pub exclude: Vec<BadgeSelector>,
    pub style: BadgeStyle,
    pub max: usize,
    /// Hex color for every badge's label, for a uniform palette.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_color: Option<String>,
    /// Hex color for every badge's message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

impl BadgeSettings {
    pub fn look(&self) -> Look<'_> {
        Look {
            style: self.style,
            label_color: self.label_color.as_deref(),
            color: self.color.as_deref(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct NavSettings {
    pub show: bool,
    pub max: usize,
    pub style: NavStyle,
    pub separator: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TocSettings {
    pub style: TocStyle,
    pub depth: u8,
    pub min_sections: usize,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct CalloutSettings {
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct CollapseSettings {
    /// Regular expressions matched (case-insensitively, whole title) against section titles.
    pub sections: Vec<String>,
    pub min_lines: usize,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct BackToTopSettings {
    pub enabled: bool,
    pub min_lines: usize,
    pub label: String,
}

/// A fully resolved theme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Theme {
    pub name: String,
    pub description: String,
    pub layout: Layout,
    pub logo: LogoSettings,
    pub badges: BadgeSettings,
    pub nav: NavSettings,
    pub toc: TocSettings,
    pub callouts: CalloutSettings,
    pub collapse: CollapseSettings,
    pub back_to_top: BackToTopSettings,
}

impl Theme {
    /// Collapse patterns, compiled to match whole section titles case-insensitively.
    pub(crate) fn collapse_patterns(&self) -> Vec<Regex> {
        self.collapse
            .sections
            .iter()
            .filter_map(|p| whole_title(p).ok())
            .collect()
    }

    fn validate(&self) -> Result<(), String> {
        for pattern in &self.collapse.sections {
            whole_title(pattern).map_err(|e| format!("collapse pattern '{pattern}': {e}"))?;
        }
        if !(1..=6).contains(&self.toc.depth) {
            return Err("toc.depth must be between 1 and 6".into());
        }
        let hex = |c: &str| (3..=8).contains(&c.len()) && c.chars().all(|ch| ch.is_ascii_hexdigit());
        for color in [&self.badges.label_color, &self.badges.color]
            .into_iter()
            .flatten()
        {
            if !hex(color) {
                return Err(format!(
                    "badge color '{color}' must be a hex color such as 1f2328"
                ));
            }
        }
        Ok(())
    }
}

fn whole_title(pattern: &str) -> Result<Regex, regex::Error> {
    Regex::new(&format!(r"(?i)^\W*(?:{pattern})\W*$"))
}

/// Why a theme could not be resolved.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ThemeError {
    #[error("unknown theme '{name}' (available: {available})")]
    Unknown { name: String, available: String },
    #[error("theme '{0}' extends itself through a cycle")]
    Cycle(String),
    #[error("'{0}' is a built-in theme; give the custom theme another name")]
    Shadowed(String),
    #[error("theme '{name}': {message}")]
    Invalid { name: String, message: String },
}

/// Built-in and custom themes, unresolved.
#[derive(Debug, Clone)]
pub struct Themes {
    tables: BTreeMap<String, Table>,
    order: Vec<String>,
}

impl Default for Themes {
    fn default() -> Self {
        Self::builtin()
    }
}

impl Themes {
    pub fn builtin() -> Self {
        let mut themes = Self {
            tables: BTreeMap::new(),
            order: Vec::new(),
        };
        for (name, source) in BUILTIN {
            let table: Table = source.parse().expect("built-in themes are valid TOML");
            themes.tables.insert((*name).to_owned(), table);
            themes.order.push((*name).to_owned());
        }
        themes
    }

    /// Add a custom theme. It extends `base` unless it says otherwise.
    pub fn define(&mut self, name: &str, table: Table) -> Result<(), ThemeError> {
        if BUILTIN.iter().any(|(builtin, _)| *builtin == name) {
            return Err(ThemeError::Shadowed(name.to_owned()));
        }
        if !self.tables.contains_key(name) {
            self.order.push(name.to_owned());
        }
        self.tables.insert(name.to_owned(), table);
        Ok(())
    }

    /// Selectable theme names: built-ins first, then custom themes, in definition order.
    pub fn names(&self) -> Vec<&str> {
        self.order
            .iter()
            .map(String::as_str)
            .filter(|n| *n != BASE)
            .collect()
    }

    pub fn contains(&self, name: &str) -> bool {
        name != BASE && self.tables.contains_key(name)
    }

    /// Resolve `name` as defined, without overrides.
    pub fn get(&self, name: &str) -> Result<Theme, ThemeError> {
        self.resolve(name, &Table::new())
    }

    /// Resolve `name` with `overrides` (configuration tables) merged on top.
    pub fn resolve(&self, name: &str, overrides: &Table) -> Result<Theme, ThemeError> {
        if !self.contains(name) {
            return Err(ThemeError::Unknown {
                name: name.to_owned(),
                available: self.names().join(", "),
            });
        }
        let mut chain = Vec::new();
        let mut current = name.to_owned();
        while current != BASE {
            if chain.contains(&current) {
                return Err(ThemeError::Cycle(name.to_owned()));
            }
            let table = self.tables.get(&current).ok_or_else(|| ThemeError::Unknown {
                name: current.clone(),
                available: self.names().join(", "),
            })?;
            chain.push(current.clone());
            current = table
                .get("extends")
                .and_then(Value::as_str)
                .unwrap_or(BASE)
                .to_owned();
        }
        let mut merged = self.tables[BASE].clone();
        for link in chain.iter().rev() {
            merge(&mut merged, &self.tables[link]);
        }
        merge(&mut merged, overrides);
        merged.remove("extends");
        merged.insert("name".into(), Value::String(name.to_owned()));
        let theme: Theme =
            Value::Table(merged)
                .try_into()
                .map_err(|e: toml::de::Error| ThemeError::Invalid {
                    name: name.to_owned(),
                    message: e.message().to_owned(),
                })?;
        theme.validate().map_err(|message| ThemeError::Invalid {
            name: name.to_owned(),
            message,
        })?;
        Ok(theme)
    }
}

/// Deep-merge `overlay` into `target`: tables merge, everything else is replaced.
fn merge(target: &mut Table, overlay: &Table) {
    for (key, value) in overlay {
        match (target.get_mut(key), value) {
            (Some(Value::Table(existing)), Value::Table(incoming)) => merge(existing, incoming),
            _ => {
                target.insert(key.clone(), value.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(text: &str) -> Table {
        text.parse().expect("valid TOML")
    }

    #[test]
    fn every_builtin_theme_resolves() {
        let themes = Themes::builtin();
        assert_eq!(
            themes.names(),
            ["minimal", "startup", "ml-research", "cli", "docs"]
        );
        for name in themes.names() {
            let theme = themes.resolve(name, &Table::new()).expect(name);
            assert_eq!(theme.name, name);
            assert!(!theme.description.is_empty(), "{name} needs a description");
        }
    }

    #[test]
    fn overrides_merge_and_are_checked() {
        let themes = Themes::builtin();
        let theme = themes
            .resolve(
                "startup",
                &table("[badges]\nstyle = \"flat\"\n[toc]\nstyle = \"details\""),
            )
            .expect("valid");
        assert_eq!(
            (theme.badges.style, theme.toc.style),
            (BadgeStyle::Flat, TocStyle::Details)
        );
        assert!(theme.nav.show, "untouched settings come from the theme");

        let unknown = themes
            .resolve("minimal", &table("colour = \"red\""))
            .expect_err("unknown key");
        assert!(
            unknown.to_string().contains("unknown field `colour`"),
            "{unknown}"
        );
        let bad_value = themes
            .resolve("minimal", &table("[toc]\nstyle = \"sidebar\""))
            .expect_err("bad value");
        assert!(bad_value.to_string().contains("sidebar"), "{bad_value}");
        let bad_badge = themes
            .resolve("minimal", &table("[badges]\ninclude = [\"neon\"]"))
            .expect_err("bad badge");
        assert!(
            bad_badge.to_string().contains("unknown badge or group 'neon'"),
            "{bad_badge}"
        );
        let bad_pattern = themes
            .resolve("minimal", &table("[collapse]\nsections = [\"(\"]"))
            .expect_err("bad regex");
        assert!(
            bad_pattern.to_string().contains("collapse pattern"),
            "{bad_pattern}"
        );
    }

    #[test]
    fn custom_themes_extend_others() {
        let mut themes = Themes::builtin();
        themes
            .define(
                "brand",
                table("extends = \"startup\"\ndescription = \"Ours\"\n[badges]\ncolor = \"ff6600\""),
            )
            .expect("ok");
        let theme = themes.resolve("brand", &Table::new()).expect("valid");
        assert_eq!(theme.layout.align, Align::Center);
        assert_eq!(theme.badges.color.as_deref(), Some("ff6600"));
        assert_eq!(themes.names().last(), Some(&"brand"));
        assert!(matches!(
            themes.define("startup", Table::new()),
            Err(ThemeError::Shadowed(_))
        ));
        themes.define("a", table("extends = \"b\"")).expect("ok");
        themes.define("b", table("extends = \"a\"")).expect("ok");
        assert!(matches!(
            themes.resolve("a", &Table::new()),
            Err(ThemeError::Cycle(_))
        ));
    }

    #[test]
    fn collapse_patterns_match_whole_titles() {
        let theme = Themes::builtin()
            .resolve("startup", &Table::new())
            .expect("valid");
        let patterns = theme.collapse_patterns();
        let matches = |title: &str| patterns.iter().any(|p| p.is_match(title));
        assert!(matches("📜 Changelog"));
        assert!(matches("What's New"));
        assert!(!matches("Changelog policy and process"));
    }
}
