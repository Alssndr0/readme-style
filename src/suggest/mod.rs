//! `readme-style suggest`: ask a model for a tagline or an architecture diagram.
//!
//! Suggestions are printed, never written: once you paste one into the README it is your
//! text, and readme-style keeps it like any other. This is the only part of readme-style that
//! uses the network, and only when you run it.

mod llm;
mod prompts;

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anstyle::Style;
use readme_style_core::config;
use readme_style_core::project::{OsFs, inspect};
use serde::{Deserialize, Serialize};

use crate::args::{Format, Provider, SuggestArgs, SuggestKind};
use crate::files::find_readme;
use llm::{Backend, Credential, Effort, LlmError, Model};

const DEFAULT_ANTHROPIC_MODEL: &str = "claude-opus-5-5";
const DEFAULT_ANTHROPIC_URL: &str = "https://api.anthropic.com";
const DEFAULT_OPENAI_URL: &str = "https://api.openai.com/v1";

/// The `[ai]` table of the configuration. A configuration file travels with the repository, so
/// it may choose the provider and the model but never where requests go or which environment
/// variable holds the key: a cloned repository must not be able to send your key elsewhere.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct AiSettings {
    provider: Option<Provider>,
    model: Option<String>,
}

const UNSAFE_KEYS: &[&str] = &["base-url", "api-key-env", "api-key"];

#[derive(Debug, Serialize)]
struct Report {
    kind: &'static str,
    provider: &'static str,
    model: String,
    suggestions: Vec<String>,
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

/// Which model to ask: flags first, then the `[ai]` configuration, then the environment.
///
/// `--base-url` sets aside the `[ai]` table: the host comes from the command line, so the
/// credential sent to it must too. Otherwise a repository saying `provider = "anthropic"` would
/// send your Anthropic key to whatever server you meant for a local model.
fn resolve(args: &SuggestArgs, settings: &AiSettings) -> Result<Model, String> {
    let base_url = args.base_url.clone();
    let ignored = AiSettings::default();
    let settings = if base_url.is_some() { &ignored } else { settings };
    let provider = args.provider.or(settings.provider).or_else(|| {
        if base_url.is_some() {
            Some(Provider::Openai)
        } else if env("ANTHROPIC_API_KEY").is_some() || env("ANTHROPIC_AUTH_TOKEN").is_some() {
            Some(Provider::Anthropic)
        } else if env("OPENAI_BASE_URL").is_some() || env("OPENAI_API_KEY").is_some() {
            Some(Provider::Openai)
        } else {
            None
        }
    });
    let model = args.model.clone().or_else(|| settings.model.clone());
    match provider {
        None => Err(
            "no model configured. Set ANTHROPIC_API_KEY, or point --base-url at an OpenAI-compatible server \
                     (a local vLLM, Ollama or LM Studio works), or add an [ai] table to the configuration"
                .into(),
        ),
        Some(Provider::Anthropic) => {
            let credential = match env("ANTHROPIC_API_KEY") {
                Some(key) => Credential::ApiKey(key),
                None => Credential::AuthToken(
                    env("ANTHROPIC_AUTH_TOKEN")
                        .ok_or("set ANTHROPIC_API_KEY (or ANTHROPIC_AUTH_TOKEN) for Anthropic")?,
                ),
            };
            let base_url = base_url
                .or_else(|| env("ANTHROPIC_BASE_URL"))
                .unwrap_or_else(|| DEFAULT_ANTHROPIC_URL.into());
            Ok(Model {
                backend: Backend::Anthropic { base_url, credential },
                name: model.unwrap_or_else(|| DEFAULT_ANTHROPIC_MODEL.into()),
            })
        }
        Some(Provider::Openai) => {
            let api_key = env("OPENAI_API_KEY");
            let base_url = base_url
                .or_else(|| env("OPENAI_BASE_URL"))
                .unwrap_or_else(|| DEFAULT_OPENAI_URL.into());
            let name = match model {
                Some(name) => name,
                None => llm::only_model(&base_url, api_key.as_deref())
                    .map_err(|e| e.to_string())?
                    .ok_or("pass --model: the server offers more than one model, or does not list them")?,
            };
            Ok(Model {
                backend: Backend::OpenAiCompatible { base_url, api_key },
                name,
            })
        }
    }
}

fn readme_path(path: Option<&PathBuf>) -> Result<PathBuf, String> {
    let raw = path.cloned().unwrap_or_else(|| PathBuf::from("."));
    let readme = find_readme(&raw).ok_or_else(|| format!("no README found at {}", raw.display()))?;
    fs::canonicalize(&readme).map_err(|e| format!("cannot resolve {}: {e}", readme.display()))
}

fn run(args: &SuggestArgs) -> Result<(Report, String), String> {
    let readme = readme_path(args.path.as_ref())?;
    let text = fs::read_to_string(&readme).map_err(|e| format!("cannot read {}: {e}", readme.display()))?;
    let config = config::discover(&OsFs, &readme).map_err(|e| e.to_string())?;
    if let Some(key) = UNSAFE_KEYS.iter().find(|k| config.ai.contains_key(**k)) {
        return Err(format!(
            "[ai] {key} cannot be set in a project's configuration, which comes with the repository: \
             use --base-url, or ANTHROPIC_BASE_URL, OPENAI_BASE_URL and the standard key variables"
        ));
    }
    let settings: AiSettings = toml::Value::Table(config.ai.clone())
        .try_into()
        .map_err(|e: toml::de::Error| format!("[ai] configuration: {}", e.message()))?;
    let model = resolve(args, &settings)?;
    let facts = inspect(&OsFs, &readme, &config.project_overrides());
    let (kind, answer) = match args.kind {
        SuggestKind::Tagline => {
            let answer = llm::complete(
                &model,
                prompts::TAGLINE_SYSTEM,
                &prompts::tagline_prompt(&text, &facts),
                Effort::Low,
            );
            ("tagline", answer.map(|a| prompts::parse_taglines(&a)))
        }
        SuggestKind::Diagram => {
            let root = readme.parent().unwrap_or(Path::new("."));
            let prompt = prompts::diagram_prompt(&text, &facts, &prompts::file_tree(root));
            let answer = llm::complete(&model, prompts::DIAGRAM_SYSTEM, &prompt, Effort::Medium);
            (
                "diagram",
                answer.map(|a| prompts::parse_diagram(&a).into_iter().collect()),
            )
        }
    };
    let suggestions = answer.map_err(|e: LlmError| format!("{} ({}): {e}", model.name, model.provider()))?;
    if suggestions.is_empty() {
        return Err(format!(
            "{} gave no usable {kind}; try again or another model",
            model.name
        ));
    }
    let report = Report {
        kind,
        provider: model.provider(),
        model: model.name,
        suggestions,
    };
    Ok((report, readme.display().to_string()))
}

pub(crate) fn main(args: &SuggestArgs) -> ExitCode {
    let mut out = anstream::stdout();
    let (report, readme) = match run(args) {
        Ok(found) => found,
        Err(message) => {
            let red = anstyle::AnsiColor::Red.on_default();
            let _ = writeln!(anstream::stderr(), "{red}error{red:#}: {message}");
            return ExitCode::from(2);
        }
    };
    if args.format == Format::Json {
        let _ = writeln!(
            out,
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        );
        return ExitCode::SUCCESS;
    }
    let bold = Style::new().bold();
    let dim = Style::new().dimmed();
    let _ = writeln!(
        out,
        "{bold}{} for {readme}{bold:#} {dim}({} via {}){dim:#}\n",
        title(args.kind),
        report.model,
        report.provider
    );
    if args.kind == SuggestKind::Tagline {
        for (i, tagline) in report.suggestions.iter().enumerate() {
            let _ = writeln!(out, "  {}. {tagline}", i + 1);
        }
        let _ = writeln!(
            out,
            "\n{dim}Paste the one you like under the title. From then on it is your text.{dim:#}"
        );
    } else {
        let _ = writeln!(out, "```mermaid\n{}\n```", report.suggestions[0]);
        let _ = writeln!(
            out,
            "\n{dim}Check it against the code, then paste it into a section such as \"## Architecture\".{dim:#}"
        );
    }
    ExitCode::SUCCESS
}

fn title(kind: SuggestKind) -> &'static str {
    match kind {
        SuggestKind::Tagline => "Tagline suggestions",
        SuggestKind::Diagram => "Architecture diagram",
    }
}
