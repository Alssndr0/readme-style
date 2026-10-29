//! GitHub Actions workflows: names, triggers, and what they publish.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use super::{FileSystem, Registry, Workflow};

static ON: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?m)^(?:on|"on"|'on'|true)\s*:(.*)$"#).expect("valid regex"));
static NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?m)^name:\s*["']?(.+?)["']?\s*$"#).expect("valid regex"));
static KEY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*([\w-]+)\s*:").expect("valid regex"));

/// The events in a workflow's `on:` key: a scalar, a flow list, a block list or a mapping.
pub fn parse_triggers(text: &str) -> BTreeSet<String> {
    let Some(captures) = ON.captures(text) else {
        return BTreeSet::new();
    };
    let inline = captures[1].split('#').next().unwrap_or_default().trim();
    if !inline.is_empty() {
        return inline
            .trim_matches(['[', ']'])
            .split(',')
            .map(|t| t.trim().trim_matches(['\'', '"']).to_owned())
            .filter(|t| !t.is_empty())
            .collect();
    }
    let rest = &text[captures.get(0).map_or(0, |m| m.end())..];
    let mut triggers = BTreeSet::new();
    let mut indent: Option<usize> = None;
    for line in rest.lines().skip(1) {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let width = line.len() - line.trim_start().len();
        if width == 0 {
            break;
        }
        if *indent.get_or_insert(width) != width {
            continue;
        }
        if let Some(item) = line.trim().strip_prefix("- ") {
            triggers.insert(item.trim().trim_matches(['\'', '"']).to_owned());
        } else if let Some(key) = KEY.captures(line) {
            triggers.insert(key[1].to_owned());
        }
    }
    triggers
}

pub(super) fn read(fs: &dyn FileSystem, root: &Path) -> Vec<Workflow> {
    let folder = root.join(".github").join("workflows");
    fs.list_dir(&folder)
        .into_iter()
        .filter(|path| path.extension().is_some_and(|e| e == "yml" || e == "yaml"))
        .filter_map(|path| {
            let text = fs.read_to_string(&path)?;
            let file = path.file_name()?.to_string_lossy().into_owned();
            let stem = path.file_stem()?.to_string_lossy().into_owned();
            let name = NAME.captures(&text).map_or(stem, |c| c[1].to_owned());
            Some(Workflow {
                file,
                name,
                triggers: parse_triggers(&text),
                text,
            })
        })
        .collect()
}

static PUBLISH: LazyLock<Vec<(Registry, Regex)>> = LazyLock::new(|| {
    [
        (
            Registry::Pypi,
            r"pypa/gh-action-pypi-publish|twine\s+upload|uv\s+publish|poetry\s+publish|flit\s+publish\
              |hatch\s+publish|maturin\s+(?:publish|upload)|PyO3/maturin-action",
        ),
        (
            Registry::Npm,
            r"\b(?:npm|pnpm|yarn)\s+publish\b|changesets/action|JS-DevTools/npm-publish",
        ),
        (
            Registry::Crates,
            r"cargo\s+publish|release-plz|katyo/publish-crates",
        ),
    ]
    .into_iter()
    .map(|(registry, pattern)| {
        let pattern: String = pattern.split_whitespace().collect();
        (
            registry,
            Regex::new(&format!("(?i){pattern}")).expect("valid regex"),
        )
    })
    .collect()
});

/// Evidence that a workflow publishes to `registry`: the file and the matching command.
pub(super) fn publish_evidence(workflows: &[Workflow], registry: Registry) -> Option<String> {
    let pattern = &PUBLISH.iter().find(|(r, _)| *r == registry)?.1;
    workflows.iter().find_map(|w| {
        pattern
            .find(&w.text)
            .map(|m| format!("{} runs {}", w.file, m.as_str()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn parses_every_trigger_form() {
        assert_eq!(parse_triggers("on: push\n"), set(&["push"]));
        assert_eq!(
            parse_triggers("on: [push, pull_request]\n"),
            set(&["push", "pull_request"])
        );
        assert_eq!(
            parse_triggers("name: x\non:\n  push:\n    branches: [main]\n  pull_request:\njobs:\n"),
            set(&["push", "pull_request"])
        );
        assert_eq!(
            parse_triggers("\"on\":\n  - push\n  - workflow_dispatch\n"),
            set(&["push", "workflow_dispatch"])
        );
        assert!(parse_triggers("jobs: {}\n").is_empty());
    }

    #[test]
    fn finds_publish_steps() {
        let workflow = Workflow {
            file: "release.yml".into(),
            name: "Release".into(),
            triggers: set(&["release"]),
            text: "steps:\n  - uses: pypa/gh-action-pypi-publish@release/v1\n".into(),
        };
        assert_eq!(
            publish_evidence(&[workflow], Registry::Pypi).as_deref(),
            Some("release.yml runs pypa/gh-action-pypi-publish")
        );
    }
}
