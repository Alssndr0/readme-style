//! What `readme-style suggest` asks a model, and how its answers are read.

use std::fs;
use std::path::Path;

use readme_style_core::project::ProjectFacts;

pub(crate) const TAGLINE_SYSTEM: &str = "You write the tagline of a software project's README: the one line under the title \
that says what the project is and why someone would use it. Base it only on what the README and the project metadata \
say; never invent features, numbers, comparisons or audiences. Plain text, at most 90 characters, no emoji, no quotes, \
no markdown, no trailing period. Avoid hype such as blazing, revolutionary, ultimate, seamless or next-generation.";

pub(crate) const DIAGRAM_SYSTEM: &str = "You draw a Mermaid flowchart of a software project's architecture for its README. \
Use only components that the README or the file tree show, and leave out anything you would have to guess. Keep it small \
(4 to 12 nodes) and readable: short noun labels, edges labelled with what flows between components, `flowchart LR` or \
`flowchart TD`. Reply with one fenced mermaid code block and nothing else.";

/// Folders that never help describe a project.
const SKIPPED: &[&str] = &[
    "target",
    "node_modules",
    "dist",
    "build",
    "out",
    "vendor",
    "__pycache__",
    "venv",
];
const TREE_LIMIT: usize = 200;

fn facts_summary(facts: &ProjectFacts) -> String {
    let mut lines = Vec::new();
    if let Some(name) = &facts.name {
        lines.push(format!("name: {name}"));
    }
    if let Some(description) = &facts.description {
        lines.push(format!("description: {description}"));
    }
    for package in &facts.packages {
        lines.push(format!("package: {} on {}", package.name, package.registry));
    }
    if let Some(license) = &facts.license {
        lines.push(format!("license: {license}"));
    }
    if lines.is_empty() {
        "(none found)".to_owned()
    } else {
        lines.join("\n")
    }
}

pub(crate) fn tagline_prompt(readme: &str, facts: &ProjectFacts) -> String {
    format!(
        "Project metadata:\n{}\n\n<readme>\n{readme}\n</readme>\n\nReply with exactly three different taglines, one per line, \
         and nothing else.",
        facts_summary(facts)
    )
}

pub(crate) fn diagram_prompt(readme: &str, facts: &ProjectFacts, tree: &str) -> String {
    format!(
        "Project metadata:\n{}\n\nFile tree (two levels):\n{tree}\n\n<readme>\n{readme}\n</readme>",
        facts_summary(facts)
    )
}

/// Up to three taglines from a model's answer, with list markers and quotes removed.
pub(crate) fn parse_taglines(answer: &str) -> Vec<String> {
    answer
        .lines()
        .map(|line| {
            let line = line.trim().trim_start_matches(['-', '*', '•']).trim_start();
            let line = line
                .trim_start_matches(|c: char| c.is_ascii_digit())
                .trim_start_matches(['.', ')'])
                .trim();
            line.trim_matches(['"', '\'', '`', '“', '”'])
                .trim()
                .trim_end_matches('.')
                .to_owned()
        })
        .filter(|line| !line.is_empty() && line.chars().count() <= 120 && !line.starts_with('<'))
        .take(3)
        .collect()
}

/// The Mermaid flowchart in a model's answer, if it gave one.
pub(crate) fn parse_diagram(answer: &str) -> Option<String> {
    let start = answer.find("```mermaid")?;
    let body = &answer[start + "```mermaid".len()..];
    let body = &body[..body.find("```")?];
    let diagram = body.trim();
    let kind = diagram.split_whitespace().next()?;
    matches!(kind, "flowchart" | "graph").then(|| diagram.to_owned())
}

/// A two-level listing of `root`, without hidden and generated folders.
pub(crate) fn file_tree(root: &Path) -> String {
    let mut lines = Vec::new();
    let entries = |dir: &Path| -> Vec<(String, bool)> {
        let mut found: Vec<(String, bool)> = fs::read_dir(dir)
            .map(|read| {
                read.filter_map(Result::ok)
                    .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path().is_dir()))
                    .filter(|(name, _)| !name.starts_with('.') && !SKIPPED.contains(&name.as_str()))
                    .collect()
            })
            .unwrap_or_default();
        found.sort();
        found
    };
    for (name, is_dir) in entries(root) {
        lines.push(if is_dir { format!("{name}/") } else { name.clone() });
        if is_dir {
            for (child, child_dir) in entries(&root.join(&name)) {
                lines.push(format!("  {child}{}", if child_dir { "/" } else { "" }));
            }
        }
        if lines.len() >= TREE_LIMIT {
            lines.truncate(TREE_LIMIT);
            lines.push("  ...".into());
            break;
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taglines_are_cleaned() {
        let answer = "1. \"A tiny parser for config files.\"\n- Parse configs without surprises\n\n* Comments survive round trips\n4. extra";
        assert_eq!(
            parse_taglines(answer),
            [
                "A tiny parser for config files",
                "Parse configs without surprises",
                "Comments survive round trips"
            ]
        );
    }

    #[test]
    fn diagrams_must_be_flowcharts() {
        let answer = "Here:\n```mermaid\nflowchart LR\n  A --> B\n```\n";
        assert_eq!(parse_diagram(answer).as_deref(), Some("flowchart LR\n  A --> B"));
        assert!(parse_diagram("```mermaid\nsequenceDiagram\n```").is_none());
        assert!(parse_diagram("no code").is_none());
    }
}
