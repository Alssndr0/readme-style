//! Command-line arguments.

use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "readme-style",
    version,
    about = "Keep your README's words, upgrade its presentation.",
    long_about = "Keep your README's words, upgrade its presentation.\n\n\
        Adds badges inferred from your project, a centered header, navigation or a table of contents, \
        GitHub alerts and collapsible sections, according to a theme. It never changes what the README \
        says: every result is checked against the original before anything is written.\n\n\
        Shows a diff by default and asks before writing. Use --write to apply, --check in CI.",
    after_help = "Exit codes: 0 ok, 1 --check found changes, 2 error, 3 refused (content would change)."
)]
pub(crate) struct Args {
    /// README files or folders holding one [default: .]
    #[arg(value_name = "PATH")]
    pub(crate) paths: Vec<PathBuf>,

    /// Theme to apply [default: from config, then from the README's markers, then minimal]
    #[arg(short, long, value_name = "NAME")]
    pub(crate) theme: Option<String>,

    /// Write the changes without asking
    #[arg(short, long, group = "mode")]
    pub(crate) write: bool,

    /// Exit with 1 if a README would change, for CI and pre-commit
    #[arg(long, group = "mode")]
    pub(crate) check: bool,

    /// Print the diff and exit
    #[arg(long, group = "mode")]
    pub(crate) diff: bool,

    /// Print the styled README to stdout
    #[arg(long, group = "mode")]
    pub(crate) stdout: bool,

    /// Show what was detected about the project, and why each badge was added
    #[arg(long)]
    pub(crate) explain: bool,

    /// Output format
    #[arg(long, value_enum, default_value_t = Format::Text)]
    pub(crate) format: Format,

    /// Do not print diffs
    #[arg(short, long)]
    pub(crate) quiet: bool,

    /// When to use colors
    #[arg(long, value_enum, default_value_t = ColorWhen::Auto)]
    pub(crate) color: ColorWhen,

    /// List the available themes and exit
    #[arg(long)]
    pub(crate) list_themes: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum Format {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum ColorWhen {
    Auto,
    Always,
    Never,
}

impl ColorWhen {
    pub(crate) fn write_global(self) {
        let choice = match self {
            ColorWhen::Auto => anstream::ColorChoice::Auto,
            ColorWhen::Always => anstream::ColorChoice::Always,
            ColorWhen::Never => anstream::ColorChoice::Never,
        };
        choice.write_global();
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    #[test]
    fn arguments_are_consistent() {
        super::Args::command().debug_assert();
    }
}
