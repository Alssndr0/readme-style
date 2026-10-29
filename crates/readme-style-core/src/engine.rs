//! Styling: run the pipeline, then prove the result is safe and stable before returning it.

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::Diagnostic;
use crate::badges::Badge;
use crate::document::Document;
use crate::guard::{self, GuardViolation};
use crate::project::ProjectFacts;
use crate::source::Normalized;
use crate::theme::Theme;
use crate::transform::{Context, PIPELINE};

/// A styled README and what went into it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Outcome {
    /// The styled README, with the original line endings and byte order mark.
    pub output: String,
    pub changed: bool,
    /// The badges in the header, with the evidence for each.
    pub badges: Vec<Badge>,
    /// The pipeline steps that changed something.
    pub changed_by: Vec<&'static str>,
    /// Problems found in the README that did not stop styling.
    pub diagnostics: Vec<Diagnostic>,
}

/// Why a README was left untouched.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum StyleError {
    /// Styling would have changed what the README says.
    #[error("refusing to change the README: {0}")]
    Guard(#[from] GuardViolation),
    /// A second run would change the output again. This is a bug in readme-style.
    #[error(
        "a second run would change the README again (in step '{0}'); this is a bug in readme-style, please report it"
    )]
    NotIdempotent(&'static str),
    /// The original line endings could not be put back without changing the text. This is a
    /// bug in readme-style.
    #[error(
        "the README's line endings could not be kept without changing it; this is a bug in readme-style, please report it"
    )]
    LineEndings,
}

struct Run {
    text: String,
    changed_by: Vec<&'static str>,
    badges: Vec<Badge>,
}

/// The normalization step that removes wrapper lines; never reported as a change.
const STRIP: &str = "strip-wrappers";

fn run(source: &str, theme: &Theme, facts: &ProjectFacts) -> Run {
    let mut cx = Context {
        theme,
        facts,
        badges: Vec::new(),
    };
    let mut text = source.to_owned();
    let mut changed_by = Vec::new();
    // Parse only after a step changed the text; most steps change nothing.
    let mut doc = Document::parse(&text);
    for (name, step) in PIPELINE {
        if let Some(lines) = step(&doc, &mut cx) {
            let next = doc.render(&lines);
            if next != text {
                changed_by.push(*name);
                text = next;
                doc = Document::parse(&text);
            }
        }
    }
    if text == source {
        // Wrapper lines were stripped and rebuilt exactly: nothing changed.
        changed_by.clear();
    }
    changed_by.retain(|step| *step != STRIP);
    Run {
        text,
        changed_by,
        badges: cx.badges,
    }
}

/// Restyle `source` with `theme`, using what is known about the project.
///
/// The result is checked twice before it is returned: it must say exactly what the source
/// says (see [`guard`](crate::guard)), and styling it again must change nothing.
pub fn style(source: &str, theme: &Theme, facts: &ProjectFacts) -> Result<Outcome, StyleError> {
    let normalized = Normalized::new(source);
    let (front, body) = split_front_matter(&normalized.text);
    let offset = front.matches('\n').count();
    let first = run(body, theme, facts);
    guard::verify(body, &first.text).map_err(|mut violation| {
        violation.line = violation.line.map(|l| l + offset);
        violation
    })?;
    let styled = format!("{front}{}", first.text);
    // The second run sees the result as a new invocation would: removing a stale region at the
    // top can leave lines that now read as front matter.
    let (second_front, second_body) = split_front_matter(&styled);
    let second = run(second_body, theme, facts);
    if second_front != front {
        return Err(StyleError::NotIdempotent("front matter"));
    }
    if second.text != second_body {
        let step = second.changed_by.first().copied().unwrap_or(STRIP);
        return Err(StyleError::NotIdempotent(step));
    }
    let output = normalized.restore(&styled);
    if Normalized::new(&output).text != styled {
        return Err(StyleError::LineEndings);
    }
    let mut diagnostics = Document::parse(body).diagnostics;
    for diagnostic in &mut diagnostics {
        diagnostic.line += offset;
    }
    Ok(Outcome {
        changed: output != source,
        output,
        badges: first.badges,
        changed_by: first.changed_by,
        diagnostics,
    })
}

/// YAML front matter (a first line `---`, YAML with at least one key, then `---` or `...`) and
/// the rest. GitHub shows front matter as a table and Hugging Face reads model cards from it:
/// styling starts after it. A README may also open with a horizontal rule, so the lines in
/// between must read as YAML; two rules with nothing, or only `# Project`, between them are just
/// rules.
fn split_front_matter(text: &str) -> (&str, &str) {
    /// A mapping key: quoted, or plain (not starting with a YAML indicator), then a colon.
    static KEY: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^(?:"[^"]*"|'[^']*'|[^\s#\-\[\]{},&*!|>'"%@`][^:]*?)\s*:(\s|$)"#).expect("valid regex")
    });
    let Some(rest) = text.strip_prefix("---\n") else {
        return ("", text);
    };
    let mut end = 4;
    let mut keys = 0;
    for line in rest.split_inclusive('\n') {
        end += line.len();
        let content = line.trim_end();
        if matches!(content, "---" | "...") {
            return if keys > 0 { text.split_at(end) } else { ("", text) };
        }
        if content.trim().is_empty() || content.trim_start().starts_with('#') {
            continue;
        }
        let indented = content.starts_with([' ', '\t']);
        if KEY.is_match(content) {
            keys += 1;
        } else if keys == 0 || !(indented || content.starts_with("- ")) {
            return ("", text);
        }
    }
    ("", text)
}

/// One pass of the pipeline without the guard or the stability check. For debugging tools
/// only: the result may say something different from `source`.
#[doc(hidden)]
pub fn style_unchecked(source: &str, theme: &Theme, facts: &ProjectFacts) -> String {
    let normalized = Normalized::new(source);
    let (front, body) = split_front_matter(&normalized.text);
    normalized.restore(&format!("{front}{}", run(body, theme, facts).text))
}

/// The theme named in a README's own markers, left there by an earlier run.
pub fn recorded_theme(source: &str) -> Option<String> {
    Document::parse(&Normalized::new(source).text)
        .regions
        .into_iter()
        .find_map(|r| r.attrs.get("theme").cloned())
}

#[cfg(test)]
mod tests {
    use super::split_front_matter;

    #[test]
    fn front_matter_is_split_off() {
        assert_eq!(
            split_front_matter("---\nlicense: mit\n---\n# T\n"),
            ("---\nlicense: mit\n---\n", "# T\n")
        );
        assert_eq!(split_front_matter("---\n---\nbody"), ("", "---\n---\nbody"));
        assert_eq!(split_front_matter("# T\n---\n"), ("", "# T\n---\n"));
        assert_eq!(
            split_front_matter("---\nnever closed\n"),
            ("", "---\nnever closed\n")
        );
        assert_eq!(
            split_front_matter("---\ntitle: x\nC++: yes\n# comment\ntags:\n  - a\n---\nText.\n"),
            (
                "---\ntitle: x\nC++: yes\n# comment\ntags:\n  - a\n---\n",
                "Text.\n"
            )
        );
        assert_eq!(
            split_front_matter("---\n# Project Name\n---\nText.\n"),
            ("", "---\n# Project Name\n---\nText.\n")
        );
        assert_eq!(
            split_front_matter("---\nA sentence, no key.\n---\n"),
            ("", "---\nA sentence, no key.\n---\n")
        );
    }
}
