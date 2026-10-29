//! Helpers shared by the integration tests.

#![allow(dead_code, unreachable_pub)]

use readme_style_core::project::{Logo, ProjectFacts};
use readme_style_core::theme::{Theme, Themes};
use readme_style_core::{Outcome, StyleError, style};

pub fn theme(name: &str) -> Theme {
    Themes::builtin().get(name).expect("built-in theme")
}

pub fn theme_with(name: &str, overrides: &str) -> Theme {
    Themes::builtin()
        .resolve(name, &overrides.parse().expect("valid TOML"))
        .expect("valid overrides")
}

pub fn run_with(source: &str, theme: &Theme, facts: &ProjectFacts) -> Outcome {
    match style(source, theme, facts) {
        Ok(outcome) => outcome,
        Err(error) => panic!("styling failed: {error}\n--- source ---\n{source}"),
    }
}

/// Style with a built-in theme and no project facts.
pub fn run(source: &str, theme_name: &str) -> String {
    run_with(source, &theme(theme_name), &ProjectFacts::default()).output
}

pub fn try_run(source: &str, theme_name: &str, facts: &ProjectFacts) -> Result<Outcome, StyleError> {
    style(source, &theme(theme_name), facts)
}

/// A project with a license, a logo pair and a GitHub repository.
pub fn facts() -> ProjectFacts {
    ProjectFacts {
        name: Some("quickparse".into()),
        description: Some("A tiny parser.".into()),
        license: Some("MIT".into()),
        license_file: Some("LICENSE".into()),
        repo: Some("acme/quickparse".into()),
        logo: Some(Logo {
            light: "assets/logo.svg".into(),
            dark: Some("assets/logo-dark.svg".into()),
        }),
        ..ProjectFacts::default()
    }
}

/// Lines that are not blank, which is what theme switches are compared on.
pub fn content_lines(text: &str) -> Vec<&str> {
    text.lines().filter(|l| !l.trim().is_empty()).collect()
}
