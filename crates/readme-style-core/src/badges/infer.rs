//! Inferring badges from project facts and from links already in the README.
//!
//! Every badge needs evidence and records it in [`Badge::reason`]. A registry badge needs
//! proof the package is published (a publish step in a workflow, or an install command in
//! the README); a paper or model badge needs a link that is clearly about this project.

use std::sync::LazyLock;

use regex::Regex;

use super::shields::Image;
use super::{Badge, BadgeKind as K};
use crate::outline::Link;
use crate::project::{Package, ProjectFacts, Registry, Tool, Workflow};
use crate::urls;

/// What badge inference reads besides the project facts.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ReadmeView<'a> {
    pub(crate) links: &'a [Link],
    pub(crate) text: &'a str,
    pub(crate) intro_end: usize,
}

fn regex(pattern: &str) -> Regex {
    Regex::new(pattern).expect("valid regex")
}

fn name_pattern(name: &str) -> String {
    name.split(['-', '_', '.'])
        .filter(|p| !p.is_empty())
        .map(regex::escape)
        .collect::<Vec<_>>()
        .join("[-_.]")
}

/// Evidence in the README text that `package` is installable from its registry.
pub(crate) fn install_evidence(package: &Package, text: &str) -> Option<String> {
    if package.registry == Registry::Go {
        return text
            .contains(&package.name)
            .then(|| format!("the README mentions {}", package.name));
    }
    let n = name_pattern(&package.name);
    let flags = r"(?:-\S+\s+)*";
    let pattern = match package.registry {
        Registry::Pypi => format!(
            r#"(?i)((?:(?:python3?\s+-m\s+)?pip3?|uv\s+pip|pipx|uv\s+tool)\s+install\s+{flags}["']?{n}|(?:uv|poetry|pdm|rye)\s+add\s+{flags}{n}|uvx\s+{n})(?:[^\w-]|$)"#
        ),
        Registry::Npm => format!(
            r"(?i)((?:npm\s+(?:install|i|add)|yarn\s+(?:global\s+)?add|pnpm\s+(?:add|install|i)|bun\s+(?:add|install|i))\s+{flags}{n}|(?:npx|bunx|pnpx)\s+{n})(?:[^\w/-]|$)"
        ),
        Registry::Crates => format!(r"(?i)(cargo\s+(?:add|install)\s+{flags}{n})(?:[^\w-]|$)"),
        Registry::Go => unreachable!("handled above"),
    };
    let captures = regex(&pattern).captures(text)?;
    Some(format!("the README shows `{}`", captures[1].trim()))
}

/// `3.10+` from `requires-python`, else from trove classifiers.
pub(crate) fn python_version(facts: &ProjectFacts) -> Option<String> {
    static LOWER: LazyLock<Regex> = LazyLock::new(|| regex(r"(?:>=|~=|\^)\s*(\d+\.\d+)"));
    static EXACT: LazyLock<Regex> = LazyLock::new(|| regex(r"==\s*(\d+\.\d+)"));
    if let Some(spec) = &facts.python_requires {
        if let Some(c) = LOWER.captures(spec) {
            return Some(format!("{}+", &c[1]));
        }
        if let Some(c) = EXACT.captures(spec) {
            return Some(c[1].to_owned());
        }
    }
    let key = |v: &String| {
        v.split('.')
            .map(|p| p.parse::<u32>().unwrap_or(0))
            .collect::<Vec<_>>()
    };
    facts
        .python_classifiers
        .iter()
        .min_by_key(|v| key(v))
        .map(|v| format!("{v}+"))
}

/// `18+` from an `engines.node` range.
pub(crate) fn node_version(spec: &str) -> Option<String> {
    static NUMBER: LazyLock<Regex> = LazyLock::new(|| regex(r"\d+"));
    let major = NUMBER.find(spec)?.as_str();
    Some(if spec.contains(['>', '^', '~', 'x', '*']) {
        format!("{major}+")
    } else {
        major.to_owned()
    })
}

static CI_SKIP: LazyLock<Regex> = LazyLock::new(|| {
    regex(
        r"(?i)release|publish|deploy|pages|stale|label|codeql|dependabot|greet|welcome|lock|triage|bump|backport|\bcla\b|semantic|pr-title|issue|docs?\b|notify|authori[sz]|comment|remind|cleanup|cache|\bbots?\b|mirror",
    )
});
static CI_EXACT: LazyLock<Regex> = LazyLock::new(|| regex(r"(?i)^(ci|tests?|build|main|checks?|pipeline)$"));
static CI_RANKS: LazyLock<[Regex; 3]> = LazyLock::new(|| {
    [
        regex(r"(?i)test"),
        regex(r"(?i)\bci\b"),
        regex(r"(?i)build|check|lint"),
    ]
});

/// The workflow that best represents "the build passes": runs on pushes or pull requests,
/// and is named like tests, CI, build or lint. `None` rather than a guess.
pub(crate) fn pick_workflow(workflows: &[Workflow]) -> Option<&Workflow> {
    let runs_on_changes = |w: &Workflow| {
        ["push", "pull_request", "pull_request_target", "merge_group"]
            .iter()
            .any(|t| w.triggers.contains(*t))
    };
    let rank = |w: &Workflow| {
        let stem = w.file.rsplit_once('.').map_or(w.file.as_str(), |(stem, _)| stem);
        if CI_EXACT.is_match(stem) || CI_EXACT.is_match(&w.name) {
            return Some(0);
        }
        let label = format!("{} {stem}", w.name);
        CI_RANKS.iter().position(|p| p.is_match(&label)).map(|i| i + 1)
    };
    workflows
        .iter()
        .filter(|w| runs_on_changes(w) && !CI_SKIP.is_match(&format!("{} {}", w.file, w.name)))
        .filter_map(|w| rank(w).map(|r| (r, w)))
        .min_by_key(|(r, _)| *r)
        .map(|(_, w)| w)
}

fn stat(label: &str, message: &str, color: &'static str) -> Image {
    Image::Static {
        label: label.to_owned(),
        message: message.to_owned(),
        color,
        logo: None,
        logo_color: None,
    }
}

fn stat_logo(
    label: &str,
    message: &str,
    color: &'static str,
    logo: &'static str,
    logo_color: &'static str,
) -> Image {
    Image::Static {
        label: label.to_owned(),
        message: message.to_owned(),
        color,
        logo: Some(logo),
        logo_color: Some(logo_color),
    }
}

fn dynamic(path: String, label: Option<&str>, logo: Option<&'static str>) -> Image {
    Image::Dynamic {
        path,
        label: label.map(str::to_owned),
        logo,
        logo_color: None,
    }
}

fn badge(
    kind: K,
    alt: impl Into<String>,
    image: Image,
    link: Option<String>,
    reason: impl Into<String>,
) -> Badge {
    Badge {
        kind,
        alt: alt.into(),
        image,
        link,
        reason: reason.into(),
    }
}

/// Every badge there is evidence for, in catalog order within each source.
pub(crate) fn candidates(facts: &ProjectFacts, readme: ReadmeView<'_>) -> Vec<Badge> {
    let mut found: Vec<Badge> = Vec::new();
    packages(facts, readme, &mut found);
    runtimes(facts, &mut found);
    project(facts, &mut found);
    research(facts, readme, &mut found);
    community(readme, &mut found);
    let mut seen = Vec::new();
    found.retain(|b| {
        let fresh = !seen.contains(&b.kind);
        seen.push(b.kind);
        fresh
    });
    found
}

fn packages(facts: &ProjectFacts, readme: ReadmeView<'_>, found: &mut Vec<Badge>) {
    for package in &facts.packages {
        let Some(evidence) = package
            .publish_evidence
            .clone()
            .or_else(|| install_evidence(package, readme.text))
        else {
            continue;
        };
        let name = &package.name;
        match package.registry {
            Registry::Pypi => {
                let why = format!("published on PyPI: {evidence}");
                found.push(badge(
                    K::Pypi,
                    "PyPI version",
                    dynamic(format!("pypi/v/{name}"), None, Some("pypi")).with_logo_color("white"),
                    Some(format!("https://pypi.org/project/{name}/")),
                    why.clone(),
                ));
                found.push(badge(
                    K::PypiDownloads,
                    "PyPI downloads",
                    dynamic(format!("pypi/dm/{name}"), None, None),
                    Some(format!(
                        "https://pypistats.org/packages/{}",
                        name.to_ascii_lowercase()
                    )),
                    why,
                ));
            }
            Registry::Npm => {
                let why = format!("published on npm: {evidence}");
                let link = Some(format!("https://www.npmjs.com/package/{name}"));
                found.push(badge(
                    K::Npm,
                    "npm version",
                    dynamic(format!("npm/v/{name}"), None, Some("npm")),
                    link.clone(),
                    why.clone(),
                ));
                found.push(badge(
                    K::NpmDownloads,
                    "npm downloads",
                    dynamic(format!("npm/dm/{name}"), None, None),
                    link,
                    why,
                ));
            }
            Registry::Crates => {
                let why = format!("published on crates.io: {evidence}");
                let link = Some(format!("https://crates.io/crates/{name}"));
                found.push(badge(
                    K::Crates,
                    "crates.io version",
                    dynamic(format!("crates/v/{name}"), None, Some("rust")),
                    link.clone(),
                    why.clone(),
                ));
                found.push(badge(
                    K::CratesDownloads,
                    "crates.io downloads",
                    dynamic(format!("crates/d/{name}"), None, None),
                    link,
                    why.clone(),
                ));
                found.push(badge(
                    K::Docsrs,
                    "docs.rs",
                    dynamic(format!("docsrs/{name}"), None, Some("docsdotrs")),
                    Some(format!("https://docs.rs/{name}")),
                    why,
                ));
            }
            Registry::Go => found.push(badge(
                K::Go,
                "Go reference",
                stat_logo("go", "reference", "007D9C", "go", "white"),
                Some(format!("https://pkg.go.dev/{name}")),
                evidence,
            )),
        }
    }
}

fn runtimes(facts: &ProjectFacts, found: &mut Vec<Badge>) {
    if let Some(version) = python_version(facts) {
        found.push(badge(
            K::Python,
            format!("Python {version}"),
            stat_logo("python", &version, "3776AB", "python", "white"),
            None,
            "requires-python or classifiers in pyproject.toml",
        ));
    }
    if let Some(version) = facts.node_engine.as_deref().and_then(node_version) {
        found.push(badge(
            K::Node,
            format!("Node.js {version}"),
            stat_logo("node", &version, "339933", "nodedotjs", "white"),
            None,
            "engines.node in package.json",
        ));
    }
    if let Some(version) = &facts.rust_version {
        found.push(badge(
            K::Rust,
            format!("Rust {version}+"),
            stat_logo("rust", &format!("{version}+"), "CE422B", "rust", "white"),
            None,
            "rust-version in Cargo.toml",
        ));
    }
    if let Some(version) = &facts.go_version {
        found.push(badge(
            K::GoVersion,
            format!("Go {version}+"),
            stat_logo("go", &format!("{version}+"), "00ADD8", "go", "white"),
            None,
            "go directive in go.mod",
        ));
    }
}

fn project(facts: &ProjectFacts, found: &mut Vec<Badge>) {
    if let Some(repo) = &facts.repo {
        if let Some(workflow) = pick_workflow(&facts.workflows) {
            let triggers: Vec<&str> = workflow.triggers.iter().map(String::as_str).collect();
            found.push(badge(
                K::Ci,
                &workflow.name,
                dynamic(
                    format!("github/actions/workflow/status/{repo}/{}", workflow.file),
                    Some(&workflow.name),
                    Some("github"),
                ),
                Some(format!(
                    "https://github.com/{repo}/actions/workflows/{}",
                    workflow.file
                )),
                format!("{} runs on {}", workflow.file, triggers.join(", ")),
            ));
        }
        if let Some(evidence) = facts.tools.get(&Tool::Codecov) {
            found.push(badge(
                K::Codecov,
                "Coverage",
                dynamic(format!("codecov/c/github/{repo}"), None, Some("codecov")),
                Some(format!("https://codecov.io/gh/{repo}")),
                evidence.clone(),
            ));
        }
    }
    let docs_url = facts.docs_url();
    if let (Some(evidence), Some(name)) = (facts.tools.get(&Tool::ReadTheDocs), &facts.name) {
        let host = docs_url.and_then(urls::host).unwrap_or_default();
        let slug = host
            .strip_suffix(".readthedocs.io")
            .map_or_else(|| name.to_ascii_lowercase(), str::to_owned);
        let link = docs_url.map_or_else(|| format!("https://{slug}.readthedocs.io"), str::to_owned);
        found.push(badge(
            K::Docs,
            "Documentation",
            dynamic(format!("readthedocs/{slug}"), None, Some("readthedocs")),
            Some(link),
            evidence.clone(),
        ));
    } else if let Some(url) = docs_url {
        found.push(badge(
            K::Docs,
            "Documentation",
            stat("docs", "online", "0A7BBB"),
            Some(url.to_owned()),
            "documentation URL in project metadata",
        ));
    }
    if let Some(license) = &facts.license {
        let source = facts.license_file.as_deref().unwrap_or("project metadata");
        found.push(badge(
            K::License,
            format!("License: {license}"),
            stat("license", license, "blue"),
            facts.license_file.clone(),
            format!("license {license} from {source}"),
        ));
    }
    let endpoint = |file: &str| Image::Endpoint {
        url: format!("https://raw.githubusercontent.com/astral-sh/{file}"),
    };
    for (tool, evidence) in &facts.tools {
        let made = match tool {
            Tool::Ruff => badge(
                K::Ruff,
                "Ruff",
                endpoint("ruff/main/assets/badge/v2.json"),
                Some("https://github.com/astral-sh/ruff".into()),
                evidence.clone(),
            ),
            Tool::Black => badge(
                K::Black,
                "Code style: black",
                stat("code style", "black", "000000"),
                Some("https://github.com/psf/black".into()),
                evidence.clone(),
            ),
            Tool::Uv => badge(
                K::Uv,
                "uv",
                endpoint("uv/main/assets/badge/v0.json"),
                Some("https://github.com/astral-sh/uv".into()),
                evidence.clone(),
            ),
            Tool::PreCommit => badge(
                K::PreCommit,
                "pre-commit",
                stat_logo("pre-commit", "enabled", "brightgreen", "pre-commit", "white"),
                Some("https://github.com/pre-commit/pre-commit".into()),
                evidence.clone(),
            ),
            Tool::Codecov | Tool::ReadTheDocs => continue,
        };
        found.push(made);
    }
}

static ARXIV: LazyLock<Regex> = LazyLock::new(|| regex(r"(?i)arxiv\.org/(?:abs|pdf)/(\d{4}\.\d{4,5})"));
static ARXIV_CITE: LazyLock<Regex> =
    LazyLock::new(|| regex(r#"(?i)(?:arXiv:|eprint\s*=\s*[{"])\s*(\d{4}\.\d{4,5})"#));
static PAPER_TEXT: LazyLock<Regex> = LazyLock::new(|| regex(r"(?i)paper|arxiv|pdf|preprint"));
static MODEL_TEXT: LazyLock<Regex> = LazyLock::new(|| regex(r"(?i)model|checkpoint|weight|hugging|🤗"));
static DEMO_TEXT: LazyLock<Regex> = LazyLock::new(|| regex(r"(?i)demo|space|try|playground"));
static DATA_TEXT: LazyLock<Regex> = LazyLock::new(|| regex(r"(?i)data"));
static PROJECT_PAGE: LazyLock<Regex> =
    LazyLock::new(|| regex(r"(?i)\b(project page|project website|website|homepage)\b"));
static DISCORD: LazyLock<Regex> = LazyLock::new(|| regex(r"(?i)(?:discord\.gg|discord\.com/invite)/\w+"));

const PAPER_HOSTS: &[(&str, &str)] = &[
    ("openreview.net", "OpenReview"),
    ("aclanthology.org", "ACL Anthology"),
    ("proceedings.neurips.cc", "NeurIPS"),
    ("openaccess.thecvf.com", "CVF"),
    ("proceedings.mlr.press", "PMLR"),
];
const HF_NOT_MODELS: &[&str] = &[
    "docs",
    "blog",
    "papers",
    "learn",
    "join",
    "pricing",
    "tasks",
    "collections",
    "models",
    "settings",
];

fn arxiv(id: &str, reason: &str) -> Badge {
    badge(
        K::Arxiv,
        format!("arXiv {id}"),
        stat_logo("arXiv", id, "B31B1B", "arxiv", "white"),
        Some(format!("https://arxiv.org/abs/{id}")),
        reason,
    )
}

fn project_page(url: &str, reason: String) -> Badge {
    badge(
        K::ProjectPage,
        "Project page",
        stat("", "Project Page", "2F80ED"),
        Some(url.to_owned()),
        reason,
    )
}

/// Paper, model and demo badges, only for links that are clearly about this project: in the
/// intro, described by their own text, or (for Hugging Face) under the GitHub owner's name.
fn research(facts: &ProjectFacts, readme: ReadmeView<'_>, found: &mut Vec<Badge>) {
    let owner = facts
        .repo
        .as_deref()
        .and_then(|r| r.split('/').next())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if let Some(c) = ARXIV_CITE.captures(readme.text) {
        found.push(arxiv(&c[1], "the README's citation names an arXiv preprint"));
    }
    for link in readme.links {
        let in_intro = link.line < readme.intro_end;
        let host = urls::host(&link.href).unwrap_or_default();
        let path = urls::path(&link.href).unwrap_or_default();
        if let Some(c) = ARXIV.captures(&link.href) {
            if in_intro || PAPER_TEXT.is_match(&link.text) {
                found.push(arxiv(&c[1], "the README links its arXiv paper"));
            }
        } else if let Some((_, venue)) = PAPER_HOSTS.iter().find(|(h, _)| *h == host) {
            if in_intro || PAPER_TEXT.is_match(&link.text) {
                found.push(badge(
                    K::Paper,
                    "Paper",
                    stat("Paper", venue, "8C1C13"),
                    Some(link.href.clone()),
                    format!("the README links its paper on {host}"),
                ));
            }
        } else if host == "huggingface.co" {
            let Some((kind, rest)) = path.split_once('/') else {
                continue;
            };
            let org = if matches!(kind, "spaces" | "datasets") {
                rest.split('/').next().unwrap_or_default()
            } else {
                kind
            };
            let (badge_kind, message, text) = match kind {
                "spaces" => (K::HfSpaces, "Demo", &*DEMO_TEXT),
                "datasets" => (K::HfDatasets, "Dataset", &*DATA_TEXT),
                _ => (K::HfModels, "Models", &*MODEL_TEXT),
            };
            let about_us = in_intro
                || text.is_match(&link.text)
                || (!owner.is_empty() && org.eq_ignore_ascii_case(&owner));
            if HF_NOT_MODELS.contains(&kind) || !about_us {
                continue;
            }
            found.push(badge(
                badge_kind,
                format!("Hugging Face {}", message.to_ascii_lowercase()),
                stat_logo("Hugging Face", message, "FFD21E", "huggingface", "000"),
                Some(link.href.clone()),
                format!(
                    "the README links a Hugging Face {} page",
                    message.to_ascii_lowercase()
                ),
            ));
        } else if host == "colab.research.google.com" {
            found.push(badge(
                K::Colab,
                "Open in Colab",
                stat_logo("Open in", "Colab", "F9AB00", "googlecolab", "white"),
                Some(link.href.clone()),
                "the README links a Colab notebook",
            ));
        } else if link.href.starts_with("http") && PROJECT_PAGE.is_match(&link.text) {
            found.push(project_page(
                &link.href,
                format!("the README links '{}'", link.text),
            ));
        }
    }
    if let Some(homepage) = facts.urls.get("homepage")
        && urls::host(homepage).is_some_and(|h| h.ends_with("github.io"))
    {
        found.push(project_page(
            homepage,
            "the project homepage is a GitHub Pages site".into(),
        ));
    }
}

fn community(readme: ReadmeView<'_>, found: &mut Vec<Badge>) {
    if let Some(link) = readme.links.iter().find(|l| DISCORD.is_match(&l.href)) {
        found.push(badge(
            K::Discord,
            "Discord",
            stat_logo("Discord", "join", "5865F2", "discord", "white"),
            Some(link.href.clone()),
            "the README links a Discord invite",
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn workflow(file: &str, name: &str, triggers: &[&str]) -> Workflow {
        Workflow {
            file: file.into(),
            name: name.into(),
            triggers: triggers.iter().map(|t| (*t).to_owned()).collect::<BTreeSet<_>>(),
            text: String::new(),
        }
    }

    #[test]
    fn install_commands_are_evidence() {
        let package = Package::new(Registry::Pypi, "quick_parse");
        assert!(install_evidence(&package, "```\npip install -U quick-parse[cli]\n```").is_some());
        assert!(install_evidence(&package, "pip install quick-parser").is_none());
        let npm = Package::new(Registry::Npm, "@acme/tidy");
        assert_eq!(
            install_evidence(&npm, "npm i -g @acme/tidy").as_deref(),
            Some("the README shows `npm i -g @acme/tidy`")
        );
    }

    #[test]
    fn ci_prefers_tests_and_never_guesses() {
        let flows = [
            workflow(
                "notify-ci.yml",
                "Notify CI authorization",
                &["pull_request_target"],
            ),
            workflow("lint.yml", "Lint", &["push"]),
            workflow("tests.yml", "Tests", &["push", "pull_request"]),
            workflow("release.yml", "Release", &["push"]),
        ];
        assert_eq!(pick_workflow(&flows).map(|w| w.file.as_str()), Some("tests.yml"));
        assert!(pick_workflow(&[workflow("buf.yml", "Buf", &["pull_request"])]).is_none());
    }

    #[test]
    fn versions() {
        let facts = ProjectFacts {
            python_requires: Some(">=3.10,<4".into()),
            ..ProjectFacts::default()
        };
        assert_eq!(python_version(&facts).as_deref(), Some("3.10+"));
        let facts = ProjectFacts {
            python_classifiers: vec!["3.12".into(), "3.9".into()],
            ..ProjectFacts::default()
        };
        assert_eq!(python_version(&facts).as_deref(), Some("3.9+"));
        assert_eq!(node_version("^18.0.0 || ^20").as_deref(), Some("18+"));
        assert_eq!(node_version("20").as_deref(), Some("20"));
    }

    #[test]
    fn research_badges_only_for_the_projects_own_links() {
        let facts = ProjectFacts {
            repo: Some("lab/model".into()),
            ..ProjectFacts::default()
        };
        let links = [
            Link {
                href: "https://arxiv.org/abs/2401.00001".into(),
                text: "Paper".into(),
                line: 2,
            },
            Link {
                href: "https://huggingface.co/lab/model-7b".into(),
                text: "Weights".into(),
                line: 2,
            },
            Link {
                href: "https://arxiv.org/abs/1810.04805".into(),
                text: "BERT".into(),
                line: 9,
            },
            Link {
                href: "https://huggingface.co/google/bert".into(),
                text: "their model".into(),
                line: 9,
            },
        ];
        let found = candidates(
            &facts,
            ReadmeView {
                links: &links,
                text: "",
                intro_end: 5,
            },
        );
        let get = |kind| {
            found
                .iter()
                .find(|b| b.kind == kind)
                .and_then(|b| b.link.as_deref())
        };
        assert_eq!(get(K::Arxiv), Some("https://arxiv.org/abs/2401.00001"));
        assert_eq!(get(K::HfModels), Some("https://huggingface.co/lab/model-7b"));
    }
}
