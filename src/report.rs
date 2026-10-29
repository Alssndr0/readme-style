//! Printing results: diffs, explanations, theme lists, JSON.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;

use anstyle::{AnsiColor, Style};
use readme_style_core::badges::Badge;
use readme_style_core::config::Config;
use readme_style_core::project::ProjectFacts;
use readme_style_core::theme::Theme;
use readme_style_core::{Diagnostic, badges};
use serde::Serialize;
use similar::TextDiff;

use crate::args::{Args, Format};

const BOLD: Style = Style::new().bold();
const DIM: Style = Style::new().dimmed();
const GREEN: Style = AnsiColor::Green.on_default();
const RED: Style = AnsiColor::Red.on_default();
const CYAN: Style = AnsiColor::Cyan.on_default();
const YELLOW: Style = AnsiColor::Yellow.on_default();

/// What happened to one README, ordered by severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Status {
    Unchanged,
    Written,
    Changed,
    Error,
    Refused,
}

impl Status {
    pub(crate) fn exit_code(self, check: bool) -> u8 {
        match self {
            Status::Unchanged | Status::Written => 0,
            Status::Changed => u8::from(check),
            Status::Error => 2,
            Status::Refused => 3,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FileReport {
    pub(crate) path: String,
    pub(crate) status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) theme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) theme_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) message: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) warnings: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) badges: Vec<Badge>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) changed_by: Vec<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) facts: Option<ProjectFacts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) diff: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) output: Option<String>,
    #[serde(skip)]
    pub(crate) hint: Option<String>,
    #[serde(skip)]
    pub(crate) previewed: bool,
}

impl FileReport {
    pub(crate) fn new(path: &Path, theme: &Theme, theme_source: &str, warnings: &[String]) -> Self {
        Self {
            theme: Some(theme.name.clone()),
            theme_source: Some(theme_source.to_owned()),
            warnings: warnings.to_vec(),
            ..Self::error(path, String::new())
        }
    }

    pub(crate) fn error(path: &Path, message: String) -> Self {
        Self {
            path: path.display().to_string(),
            status: Status::Error,
            theme: None,
            theme_source: None,
            message: (!message.is_empty()).then_some(message),
            warnings: Vec::new(),
            diagnostics: Vec::new(),
            badges: Vec::new(),
            changed_by: Vec::new(),
            facts: None,
            diff: None,
            output: None,
            hint: None,
            previewed: false,
        }
    }

    pub(crate) fn failed(mut self, status: Status, message: String) -> Self {
        self.status = status;
        self.message = Some(message);
        self
    }
}

/// A plain unified diff; colors are added when printing.
pub(crate) fn unified_diff(before: &str, after: &str, path: &Path) -> String {
    let name = path.display().to_string();
    TextDiff::from_lines(before, after)
        .unified_diff()
        .context_radius(3)
        .header(&format!("{name} (current)"), &format!("{name} (styled)"))
        .to_string()
}

fn colored_diff(diff: &str) -> String {
    let mut out = String::with_capacity(diff.len() + diff.len() / 4);
    for line in diff.split_inclusive('\n') {
        let style = if line.starts_with("+++") || line.starts_with("---") {
            BOLD
        } else if line.starts_with("@@") {
            CYAN
        } else if line.starts_with('+') {
            GREEN
        } else if line.starts_with('-') {
            RED
        } else {
            Style::new()
        };
        let body = line.strip_suffix('\n').unwrap_or(line);
        let _ = writeln!(out, "{style}{body}{style:#}");
    }
    out
}

/// Prints reports as they come (text) or all at once (JSON).
pub(crate) struct Printer {
    format: Format,
    quiet: bool,
    explain: bool,
    collected: Vec<FileReport>,
}

impl Printer {
    pub(crate) fn new(args: &Args) -> Self {
        Self {
            format: args.format,
            quiet: args.quiet,
            explain: args.explain,
            collected: Vec::new(),
        }
    }

    /// Show a diff before asking whether to apply it.
    pub(crate) fn preview(report: &FileReport) {
        if let Some(diff) = &report.diff {
            let _ = anstream::stdout().write_all(colored_diff(diff).as_bytes());
        }
    }

    pub(crate) fn themes(&mut self, config: &Config) {
        let themes: Vec<Theme> = config
            .themes
            .names()
            .iter()
            .filter_map(|name| config.themes.get(name).ok())
            .collect();
        let mut out = anstream::stdout();
        if self.format == Format::Json {
            let list: Vec<_> = themes
                .iter()
                .map(|t| serde_json::json!({"name": t.name, "description": t.description}))
                .collect();
            let _ = writeln!(out, "{}", serde_json::to_string_pretty(&list).unwrap_or_default());
            return;
        }
        let width = themes.iter().map(|t| t.name.len()).max().unwrap_or(0);
        for theme in themes {
            let _ = writeln!(out, "{BOLD}{:width$}{BOLD:#}  {}", theme.name, theme.description);
        }
    }

    pub(crate) fn file(&mut self, report: &FileReport) {
        if self.format == Format::Json {
            self.collected.push(report.clone());
            return;
        }
        let mut out = anstream::stdout();
        let mut err = anstream::stderr();
        let path = &report.path;
        for warning in &report.warnings {
            let _ = writeln!(err, "{YELLOW}warning{YELLOW:#}: {path}: {warning}");
        }
        for diagnostic in &report.diagnostics {
            let _ = writeln!(err, "{YELLOW}warning{YELLOW:#}: {path}: {diagnostic}");
        }
        if self.explain && report.facts.is_some() {
            let _ = out.write_all(explain(report).as_bytes());
        }
        if let Some(output) = &report.output {
            let _ = out.write_all(output.as_bytes());
            return;
        }
        let theme = report.theme.as_deref().unwrap_or("-");
        match report.status {
            Status::Error | Status::Refused => {
                let label = if report.status == Status::Refused {
                    "refused"
                } else {
                    "error"
                };
                let message = report.message.as_deref().unwrap_or_default();
                let _ = writeln!(err, "{RED}{label}{RED:#}: {path}: {message}");
            }
            Status::Unchanged => {
                if !self.quiet {
                    let _ = writeln!(out, "{path}: {DIM}already styled ({theme}){DIM:#}");
                }
            }
            Status::Changed | Status::Written => {
                // A preview shows the diff; a write just says so, like a formatter.
                if !self.quiet
                    && !report.previewed
                    && report.status == Status::Changed
                    && let Some(diff) = &report.diff
                {
                    let _ = out.write_all(colored_diff(diff).as_bytes());
                }
                let verb = if report.status == Status::Written {
                    "restyled"
                } else {
                    "would be restyled"
                };
                let hint = report
                    .hint
                    .as_deref()
                    .map(|h| format!(", {h}"))
                    .unwrap_or_default();
                let _ = writeln!(out, "{BOLD}{path}{BOLD:#}: {verb} ({theme}){hint}");
            }
        }
    }

    pub(crate) fn finish(&mut self) {
        if self.format == Format::Json {
            let _ = writeln!(
                anstream::stdout(),
                "{}",
                serde_json::to_string_pretty(&self.collected).unwrap_or_default()
            );
        }
    }
}

/// The `--explain` text: what was detected about the project, and why each badge is there.
fn explain(report: &FileReport) -> String {
    let Some(facts) = &report.facts else {
        return String::new();
    };
    let mut out = String::new();
    let or_dash = |value: Option<&str>| value.unwrap_or("-").to_owned();
    let mut row = |key: &str, value: String| {
        let _ = writeln!(out, "{DIM}{key:<12}{DIM:#} {value}");
    };
    row("readme", report.path.clone());
    row(
        "theme",
        format!(
            "{} (from {})",
            report.theme.as_deref().unwrap_or("-"),
            report.theme_source.as_deref().unwrap_or("-")
        ),
    );
    row("name", or_dash(facts.name.as_deref()));
    row("description", or_dash(facts.description.as_deref()));
    let repo = facts
        .repo
        .as_ref()
        .map(|r| format!("{r} (from {})", facts.repo_source.as_deref().unwrap_or("-")));
    row("repository", or_dash(repo.as_deref()));
    let license = facts.license.as_ref().map(|l| {
        format!(
            "{l}{}",
            facts
                .license_file
                .as_ref()
                .map(|f| format!(" ({f})"))
                .unwrap_or_default()
        )
    });
    row("license", or_dash(license.as_deref()));
    for package in &facts.packages {
        let status = package.publish_evidence.as_deref().map_or_else(
            || "no publish evidence found".to_owned(),
            |e| format!("published: {e}"),
        );
        row(
            "package",
            format!("{}:{} ({status})", package.registry, package.name),
        );
    }
    for workflow in &facts.workflows {
        let triggers: Vec<&str> = workflow.triggers.iter().map(String::as_str).collect();
        row(
            "workflow",
            format!("{} '{}' on {}", workflow.file, workflow.name, triggers.join(", ")),
        );
    }
    for (tool, evidence) in &facts.tools {
        row(
            "tool",
            format!(
                "{} ({evidence})",
                serde_json::to_value(tool)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default()
            ),
        );
    }
    if let Some(logo) = &facts.logo {
        row(
            "logo",
            format!(
                "{}{}",
                logo.light,
                logo.dark
                    .as_ref()
                    .map(|d| format!(" (dark: {d})"))
                    .unwrap_or_default()
            ),
        );
    }
    if report.badges.is_empty() {
        row(
            "badges",
            "none (the README already has badges, or there was no evidence for any)".into(),
        );
    }
    for badge in &report.badges {
        row(
            "badge",
            format!("{:<15} {}", badges::BadgeKind::id(badge.kind), badge.reason),
        );
    }
    out
}
