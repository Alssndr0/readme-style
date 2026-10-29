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
pub enum StyleError {
    /// Styling would have changed what the README says.
    #[error("refusing to change the README: {0}")]
    Guard(#[from] GuardViolation),
    /// A second run would change the output again. This is a bug in readme-style.
    #[error(
        "a second run would change the README again (in step '{0}'); this is a bug in readme-style, please report it"
    )]
    NotIdempotent(&'static str),
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
    let second = run(&first.text, theme, facts);
    if second.text != first.text {
        let step = second.changed_by.first().copied().unwrap_or(STRIP);
        return Err(StyleError::NotIdempotent(step));
    }
    let output = normalized.restore(&format!("{front}{}", first.text));
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

/// YAML front matter (a first line `---`, YAML, then `---` or `...`) and the rest. GitHub shows
/// front matter as a table and Hugging Face reads model cards from it: styling starts after it.
/// A README may also open with a horizontal rule, so the lines in between must read as YAML.
fn split_front_matter(text: &str) -> (&str, &str) {
    static KEY: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"^["']?[\w][\w .-]*["']?\s*:(\s|$)"#).expect("valid regex"));
    let Some(rest) = text.strip_prefix("---\n") else {
        return ("", text);
    };
    let mut end = 4;
    let mut first = true;
    for line in rest.split_inclusive('\n') {
        end += line.len();
        let content = line.trim_end();
        if matches!(content, "---" | "...") {
            return text.split_at(end);
        }
        if content.trim().is_empty() || content.trim_start().starts_with('#') {
            continue;
        }
        let indented = content.starts_with([' ', '\t']);
        let fits = KEY.is_match(content) || (!first && (indented || content.starts_with("- ")));
        if !fits {
            return ("", text);
        }
        first = false;
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
        assert_eq!(split_front_matter("---\n---\nbody"), ("---\n---\n", "body"));
        assert_eq!(split_front_matter("# T\n---\n"), ("", "# T\n---\n"));
        assert_eq!(
            split_front_matter("---\nnever closed\n"),
            ("", "---\nnever closed\n")
        );
    }
}
