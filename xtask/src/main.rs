//! Development tasks: `cargo xtask <task>`.
//!
//! * `corpus fetch [--jobs N]`: sparse-clone the repositories in `xtask/corpus.txt` into
//!   `target/corpus`.
//! * `corpus run [--filter TEXT]`: style every corpus README with every theme. Checks the
//!   guarantees (no refusals, no panics, idempotent, theme switches equal styling from
//!   scratch), writes the results to `target/corpus-out` for review, and prints a summary.
//! * `examples [--check]`: regenerate `examples/*/README.<theme>.md`, or check they are
//!   current.
//! * `screenshot <README> <PNG> [--height N]`: render through GitHub's Markdown API and
//!   screenshot with headless Chrome (needs `gh` and Chrome or Chromium).

use std::fmt::Write as _;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, bail};
use readme_style_core::config;
use readme_style_core::project::{OsFs, Overrides, ProjectFacts, inspect};
use readme_style_core::theme::{Theme, Themes};
use readme_style_core::{StyleError, style};

const README_NAMES: &[&str] = &["README.md", "readme.md", "Readme.md", "README.markdown"];
const SPARSE_PATHS: &[&str] = &[
    ".github",
    "assets",
    "docs/assets",
    "docs/images",
    "docs/img",
    "images",
    "img",
    "media",
    "static",
    "art",
    "branding",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits in the workspace")
        .to_path_buf()
}

#[allow(clippy::print_stdout, clippy::print_stderr)]
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["corpus", "fetch", rest @ ..] => corpus_fetch(rest),
        ["corpus", "run", rest @ ..] => corpus_run(rest),
        ["examples", rest @ ..] => examples(rest),
        ["screenshot", readme, png, rest @ ..] => screenshot(Path::new(readme), Path::new(png), rest),
        ["debug", readme, themes @ ..] if !themes.is_empty() => debug(Path::new(readme), themes),
        _ => {
            eprintln!(
                "usage: cargo xtask (corpus fetch|corpus run|examples [--check]|screenshot README PNG)"
            );
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::from(2)
        }
    }
}

fn flag<'a>(args: &[&'a str], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| *a == name)
        .and_then(|i| args.get(i + 1))
        .copied()
}

fn find_readme(dir: &Path) -> Option<PathBuf> {
    README_NAMES.iter().map(|n| dir.join(n)).find(|p| p.is_file())
}

// corpus fetch

fn corpus_repos() -> Result<Vec<String>> {
    let list = fs::read_to_string(root().join("xtask/corpus.txt")).context("reading xtask/corpus.txt")?;
    Ok(list
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_owned)
        .collect())
}

#[allow(clippy::print_stdout)]
fn corpus_fetch(args: &[&str]) -> Result<bool> {
    let jobs: usize = flag(args, "--jobs").map_or(Ok(8), str::parse).context("--jobs")?;
    let dir = root().join("target/corpus");
    fs::create_dir_all(&dir)?;
    let queue = Mutex::new(corpus_repos()?);
    let failed = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..jobs.max(1) {
            scope.spawn(|| {
                loop {
                    let Some(repo) = queue.lock().expect("queue").pop() else {
                        break;
                    };
                    let target = dir.join(repo.replace('/', "__"));
                    if target.exists() {
                        continue;
                    }
                    if !clone(&repo, &target) {
                        failed.lock().expect("failed").push(repo);
                    }
                }
            });
        }
    });
    let failed = failed.into_inner().expect("failed");
    println!(
        "corpus in {} ({} failed: {})",
        dir.display(),
        failed.len(),
        failed.join(", ")
    );
    Ok(failed.is_empty())
}

fn clone(repo: &str, target: &Path) -> bool {
    let url = format!("https://github.com/{repo}");
    let quiet = |command: &mut Command| {
        command
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    };
    let cloned = quiet(
        Command::new("git")
            .args([
                "clone",
                "-q",
                "--depth",
                "1",
                "--filter=blob:none",
                "--sparse",
                &url,
            ])
            .arg(target),
    );
    cloned
        && quiet(
            Command::new("git")
                .arg("-C")
                .arg(target)
                .args(["sparse-checkout", "set"])
                .args(SPARSE_PATHS),
        )
}

// corpus run

#[derive(Default)]
struct Tally {
    changed: usize,
    unchanged: usize,
    refused: Vec<String>,
    unstable: Vec<String>,
    panicked: Vec<String>,
    switch_mismatch: Vec<String>,
    time: Duration,
    runs: u32,
}

fn safe_style(text: &str, theme: &Theme, facts: &ProjectFacts) -> Result<Result<String, StyleError>, String> {
    panic::catch_unwind(AssertUnwindSafe(|| style(text, theme, facts).map(|o| o.output))).map_err(|payload| {
        payload
            .downcast_ref::<&str>()
            .map(ToString::to_string)
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "panic".into())
    })
}

#[allow(clippy::print_stdout)]
fn corpus_run(args: &[&str]) -> Result<bool> {
    let filter = flag(args, "--filter");
    let corpus = root().join("target/corpus");
    let out = root().join("target/corpus-out");
    if !corpus.is_dir() {
        bail!("no corpus yet; run `cargo xtask corpus fetch` first");
    }
    panic::set_hook(Box::new(|_| {}));
    let themes = Themes::builtin();
    let resolved: Vec<Theme> = themes
        .names()
        .iter()
        .map(|n| themes.get(n))
        .collect::<Result<_, _>>()?;
    let mut tallies: Vec<Tally> = resolved.iter().map(|_| Tally::default()).collect();
    let mut dirs: Vec<PathBuf> = fs::read_dir(&corpus)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    dirs.sort();
    let mut readmes = 0;
    for dir in dirs
        .iter()
        .filter(|d| filter.is_none_or(|f| d.to_string_lossy().contains(f)))
    {
        let Some(readme) = find_readme(dir) else { continue };
        let Ok(text) = fs::read_to_string(&readme) else {
            continue;
        };
        readmes += 1;
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let facts = inspect(&OsFs, &fs::canonicalize(&readme)?, &Overrides::default());
        let mut outputs: Vec<Option<String>> = Vec::new();
        for (theme, tally) in resolved.iter().zip(&mut tallies) {
            let start = Instant::now();
            let result = safe_style(&text, theme, &facts);
            tally.time += start.elapsed();
            tally.runs += 1;
            let output = match result {
                Ok(Ok(output)) => output,
                Ok(Err(StyleError::Guard(violation))) => {
                    tally.refused.push(format!("{name}: {violation}"));
                    outputs.push(None);
                    continue;
                }
                Ok(Err(StyleError::NotIdempotent(step))) => {
                    tally.unstable.push(format!("{name}: {step}"));
                    outputs.push(None);
                    continue;
                }
                Err(message) => {
                    tally.panicked.push(format!("{name}: {message}"));
                    outputs.push(None);
                    continue;
                }
            };
            if output == text {
                tally.unchanged += 1;
            } else {
                tally.changed += 1;
                let target = out.join(&name);
                fs::create_dir_all(&target)?;
                fs::write(target.join(format!("{}.md", theme.name)), &output)?;
            }
            outputs.push(Some(output));
        }
        // Restyling one theme's output with another must equal styling the original.
        for (a, from) in outputs.iter().enumerate() {
            let Some(from) = from else { continue };
            for (b, theme) in resolved.iter().enumerate() {
                let Some(direct) = &outputs[b] else { continue };
                let switched = safe_style(from, theme, &facts).ok().and_then(Result::ok);
                if a != b && switched.as_deref().map(without_blank_lines) != Some(without_blank_lines(direct))
                {
                    tallies[b]
                        .switch_mismatch
                        .push(format!("{name}: {} -> {}", resolved[a].name, theme.name));
                }
            }
        }
    }

    let mut report = String::new();
    let mut ok = true;
    let _ = writeln!(report, "{readmes} READMEs, outputs in {}", out.display());
    let _ = writeln!(
        report,
        "{:<12} {:>7} {:>9} {:>7} {:>8} {:>6} {:>7} {:>9}",
        "theme", "changed", "unchanged", "refused", "unstable", "panic", "switch", "mean ms"
    );
    for (theme, tally) in resolved.iter().zip(&tallies) {
        let mean = tally.time.as_secs_f64() * 1000.0 / f64::from(tally.runs.max(1));
        let _ = writeln!(
            report,
            "{:<12} {:>7} {:>9} {:>7} {:>8} {:>6} {:>7} {mean:>9.2}",
            theme.name,
            tally.changed,
            tally.unchanged,
            tally.refused.len(),
            tally.unstable.len(),
            tally.panicked.len(),
            tally.switch_mismatch.len()
        );
    }
    for (theme, tally) in resolved.iter().zip(&tallies) {
        for (label, items) in [
            ("refused", &tally.refused),
            ("unstable", &tally.unstable),
            ("panic", &tally.panicked),
            ("switch", &tally.switch_mismatch),
        ] {
            for item in items {
                ok = false;
                let _ = writeln!(report, "  [{}] {label}: {item}", theme.name);
            }
        }
    }
    print!("{report}");
    Ok(ok)
}

/// Blank lines between blocks do not change how Markdown renders. A blank line added to
/// space out a region cannot be told apart from the user's own when the region goes away,
/// so theme switches are compared without them.
fn without_blank_lines(text: &str) -> Vec<&str> {
    text.lines().filter(|l| !l.trim().is_empty()).collect()
}

// examples

#[allow(clippy::print_stdout)]
fn examples(args: &[&str]) -> Result<bool> {
    let check = args.contains(&"--check");
    let mut current = true;
    let mut dirs: Vec<PathBuf> = fs::read_dir(root().join("examples"))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    dirs.sort();
    for dir in dirs.into_iter().filter(|d| d.join("README.md").is_file()) {
        let readme = fs::canonicalize(dir.join("README.md"))?;
        let text = fs::read_to_string(&readme)?;
        let config = config::discover(&OsFs, &readme)?;
        let facts = inspect(&OsFs, &readme, &config.project_overrides());
        for name in Themes::builtin().names() {
            let theme = config.theme(name)?;
            let output = style(&text, &theme, &facts)?.output;
            let target = dir.join(format!("README.{name}.md"));
            let relative = target
                .strip_prefix(root())
                .unwrap_or(&target)
                .display()
                .to_string();
            if check {
                if fs::read_to_string(&target).ok().as_deref() != Some(output.as_str()) {
                    println!("out of date: {relative}");
                    current = false;
                }
            } else {
                fs::write(&target, output)?;
                println!("{relative}");
            }
        }
    }
    if check && !current {
        println!("run `cargo xtask examples` and review the diff");
    }
    Ok(current)
}

// debug

/// Style `readme` through each theme in turn without checks, printing every intermediate
/// result, then show what a second pass of the last theme would change.
#[allow(clippy::print_stdout)]
fn debug(readme: &Path, names: &[&str]) -> Result<bool> {
    let readme = fs::canonicalize(find_readme(readme).unwrap_or_else(|| readme.to_path_buf()))?;
    let facts = inspect(&OsFs, &readme, &Overrides::default());
    let themes = Themes::builtin();
    let mut text = fs::read_to_string(&readme)?;
    let mut last = None;
    for name in names {
        let theme = themes.get(name)?;
        text = readme_style_core::style_unchecked(&text, &theme, &facts);
        let path = std::env::temp_dir().join(format!("readme-style-debug-{name}.md"));
        fs::write(&path, &text)?;
        println!("{name}: {}", path.display());
        last = Some(theme);
    }
    let theme = last.context("no theme")?;
    let again = readme_style_core::style_unchecked(&text, &theme, &facts);
    if again == text {
        println!("a second pass changes nothing");
        return Ok(true);
    }
    let path = std::env::temp_dir().join("readme-style-debug-again.md");
    fs::write(&path, &again)?;
    println!(
        "a second pass changes the result: diff {} against the last file above",
        path.display()
    );
    Ok(false)
}

// screenshot

const PAGE: &str = r#"<!doctype html><html><head><meta charset="utf-8">
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/github-markdown-css@5/github-markdown-light.css">
<style>body{margin:0} .markdown-body{box-sizing:border-box;max-width:980px;margin:0 auto;padding:45px}</style>
</head><body><article class="markdown-body">BODY</article></body></html>"#;

/// Render with GitHub's API in "gfm" mode, the only mode that renders alerts. It also turns
/// single newlines inside paragraphs into line breaks, unlike a README on github.com.
#[allow(clippy::print_stdout)]
fn screenshot(readme: &Path, png: &Path, args: &[&str]) -> Result<bool> {
    let height: u32 = flag(args, "--height")
        .map_or(Ok(1400), str::parse)
        .context("--height")?;
    let text = fs::read_to_string(readme).with_context(|| format!("reading {}", readme.display()))?;
    let payload = serde_json::json!({ "text": text, "mode": "gfm" }).to_string();
    let mut child = Command::new("gh")
        .args(["api", "markdown", "--input", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("running gh")?;
    std::io::Write::write_all(&mut child.stdin.take().context("gh stdin")?, payload.as_bytes())?;
    let rendered = child.wait_with_output()?;
    if !rendered.status.success() {
        bail!("gh api markdown failed");
    }
    let html = readme.with_file_name(format!(
        ".{}.preview.html",
        readme
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    ));
    fs::write(
        &html,
        PAGE.replace("BODY", &String::from_utf8_lossy(&rendered.stdout)),
    )?;
    let chrome = ["google-chrome", "chromium", "chromium-browser"]
        .into_iter()
        .find(|c| {
            Command::new(c)
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok()
        })
        .context("Chrome or Chromium is required")?;
    let png = std::path::absolute(png)?;
    let status = Command::new(chrome)
        .args([
            "--headless",
            "--no-sandbox",
            "--hide-scrollbars",
            "--virtual-time-budget=8000",
        ])
        .arg(format!("--screenshot={}", png.display()))
        .arg(format!("--window-size=1000,{height}"))
        .arg(format!("file://{}", std::path::absolute(&html)?.display()))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    let _ = fs::remove_file(&html);
    println!("{}", png.display());
    Ok(status.success())
}
