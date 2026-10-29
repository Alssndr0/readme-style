//! What the command does for each README.

use std::fs;
use std::io::{self, BufRead, IsTerminal, Write};
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use readme_style_core::config::{self, Config};
use readme_style_core::project::{OsFs, ProjectFacts, inspect};
use readme_style_core::theme::{DEFAULT_THEME, Theme};
use readme_style_core::{Outcome, StyleError, recorded_theme, style};

use crate::args::{Args, Format};
use crate::files::{find_readme, resolve_readme, write_readme};
use crate::report::{self, FileReport, Printer, Status};

/// Everything needed to style one README.
struct Job {
    path: PathBuf,
    text: String,
    theme: Theme,
    theme_source: String,
    facts: ProjectFacts,
    warnings: Vec<String>,
}

pub(crate) fn main(args: &Args) -> ExitCode {
    let mut printer = Printer::new(args);
    if args.list_themes {
        let config = find_readme(Path::new("."))
            .and_then(|r| load_config(&r).ok())
            .unwrap_or_default();
        printer.themes(&config);
        return ExitCode::SUCCESS;
    }
    let paths = if args.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        args.paths.clone()
    };
    let mut status = Status::Unchanged;
    for raw in paths {
        let report = match find_readme(&raw) {
            Some(path) => process(&path, args),
            None => FileReport::error(&raw, format!("no README found at {}", raw.display())),
        };
        status = status.max(report.status);
        printer.file(&report);
    }
    printer.finish();
    ExitCode::from(status.exit_code(args.check))
}

fn load_config(readme: &Path) -> Result<Config, config::ConfigError> {
    let absolute = fs::canonicalize(readme).unwrap_or_else(|_| readme.to_path_buf());
    config::discover(&OsFs, &absolute)
}

fn prepare(path: &Path, args: &Args) -> Result<Job, String> {
    resolve_readme(path).map_err(|e| format!("will not read {}: {e}", path.display()))?;
    let bytes = fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let text = String::from_utf8(bytes).map_err(|_| format!("{} is not valid UTF-8", path.display()))?;
    let absolute = fs::canonicalize(path).map_err(|e| format!("cannot resolve {}: {e}", path.display()))?;
    let config = config::discover(&OsFs, &absolute).map_err(|e| e.to_string())?;
    let mut warnings = Vec::new();
    let (name, theme_source) = if let Some(name) = &args.theme {
        (name.clone(), "--theme".to_owned())
    } else if let Some(name) = &config.theme {
        (
            name.clone(),
            config.source.clone().unwrap_or_else(|| "config".into()),
        )
    } else if let Some(name) = recorded_theme(&text) {
        if config.themes.contains(&name) {
            (name, format!("markers in {}", path.display()))
        } else {
            warnings.push(format!(
                "the README's markers name theme '{name}', which does not exist; using {DEFAULT_THEME}"
            ));
            (DEFAULT_THEME.to_owned(), "default".to_owned())
        }
    } else {
        (DEFAULT_THEME.to_owned(), "default".to_owned())
    };
    let theme = config.theme(&name).map_err(|e| e.to_string())?;
    let facts = inspect(&OsFs, &absolute, &config.project_overrides());
    Ok(Job {
        path: path.to_path_buf(),
        text,
        theme,
        theme_source,
        facts,
        warnings,
    })
}

/// Style with the engine, turning a panic (a bug) into an error report instead of a crash.
fn style_safely(job: &Job) -> Result<Result<Outcome, StyleError>, String> {
    panic::catch_unwind(AssertUnwindSafe(|| style(&job.text, &job.theme, &job.facts))).map_err(|payload| {
        let message = payload
            .downcast_ref::<&str>()
            .map(ToString::to_string)
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown panic".into());
        format!("internal error ({message}); please report this at https://github.com/Alssndr0/readme-style/issues")
    })
}

fn process(path: &Path, args: &Args) -> FileReport {
    let job = match prepare(path, args) {
        Ok(job) => job,
        Err(message) => return FileReport::error(path, message),
    };
    let mut report = FileReport::new(&job.path, &job.theme, &job.theme_source, &job.warnings);
    if args.explain {
        report.facts = Some(job.facts.clone());
    }
    let outcome = match style_safely(&job) {
        Err(message) => return report.failed(Status::Error, message),
        Ok(Err(refusal)) => return report.failed(Status::Refused, refusal.to_string()),
        Ok(Ok(outcome)) => outcome,
    };
    report.badges.clone_from(&outcome.badges);
    report.changed_by.clone_from(&outcome.changed_by);
    report.diagnostics.clone_from(&outcome.diagnostics);
    if args.stdout {
        report.output = Some(outcome.output.clone());
    }
    if !outcome.changed {
        report.status = Status::Unchanged;
        return report;
    }
    report.diff = Some(report::unified_diff(&job.text, &outcome.output, &job.path));
    report.status = Status::Changed;
    let apply = if args.write {
        true
    } else if args.check || args.diff || args.stdout || args.format == Format::Json {
        false
    } else if io::stdin().is_terminal() && io::stdout().is_terminal() {
        Printer::preview(&report);
        report.previewed = true;
        confirm(&job.path)
    } else {
        report.hint = Some("run with --write to apply".into());
        false
    };
    if apply {
        if let Err(e) = write_readme(&job.path, &outcome.output, &job.text) {
            return report.failed(Status::Error, format!("cannot write {}: {e}", job.path.display()));
        }
        report.status = Status::Written;
    }
    report
}

fn confirm(path: &Path) -> bool {
    let mut out = io::stdout();
    let _ = write!(out, "Apply these changes to {}? [y/N] ", path.display());
    let _ = out.flush();
    let mut answer = String::new();
    io::stdin().lock().read_line(&mut answer).is_ok()
        && matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}
