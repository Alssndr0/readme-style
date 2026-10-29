//! The content guarantee: styling may add decoration, never change what the README says.
//!
//! Both versions are stripped of everything readme-style generates, rendered to HTML the way
//! GitHub renders them, and reduced to a sequence of atoms: words, code (exact), link
//! targets, image sources with alt text, heading levels, line breaks and block boundaries. The
//! sequences must be equal.
//!
//! Two presentation changes are allowed by construction. A GitHub alert renders its kind as a
//! title ("Note"), which stands in for the "**Note:**" lead-in it replaced. Emoji are not
//! words, so dropping one from a callout lead-in is not a change.

use std::fmt;
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::document::Document;
use crate::html::{Tag, tokenize};
use crate::markdown;

static WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\w+").expect("valid regex"));
static SHORTCODE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r":[a-z0-9_+-]+:").expect("valid regex"));

const BLOCKS: &[&str] = &[
    "p",
    "li",
    "ul",
    "ol",
    "pre",
    "blockquote",
    "table",
    "thead",
    "tbody",
    "tr",
    "td",
    "th",
    "hr",
    "div",
    "details",
    "summary",
    "dl",
    "dt",
    "dd",
    "section",
    "article",
    "figure",
    "figcaption",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
];

#[derive(Debug, Clone, PartialEq, Eq)]
enum Atom {
    Word(String),
    Code(String),
    Link(String),
    Image { src: String, alt: String },
    Heading(String),
    Break,
    LineBreak,
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Atom::Word(w) => f.write_str(w),
            Atom::Code(c) => write!(f, "`{}`", c.trim_end()),
            Atom::Link(href) => write!(f, "<link {href}>"),
            Atom::Image { src, .. } => write!(f, "<image {src}>"),
            Atom::Heading(tag) => write!(f, "<{tag}>"),
            Atom::Break => f.write_str("/"),
            Atom::LineBreak => f.write_str("<br>"),
        }
    }
}

/// Where and how a styled README would differ from the original.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
pub struct GuardViolation {
    /// 1-based line in the original README near the first difference.
    pub line: Option<usize>,
    /// Content that would disappear.
    pub removed: String,
    /// Content that would appear.
    pub added: String,
}

impl fmt::Display for GuardViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(line) = self.line {
            write!(f, "near line {line}, ")?;
        }
        f.write_str("the styled README would ")?;
        match (self.removed.is_empty(), self.added.is_empty()) {
            (false, false) => write!(f, "replace \"{}\" with \"{}\"", self.removed, self.added),
            (false, true) => write!(f, "drop \"{}\"", self.removed),
            _ => write!(f, "add \"{}\"", self.added),
        }
    }
}

/// Atoms with the original line each came from.
fn atoms(text: &str) -> Vec<(Atom, Option<usize>)> {
    let doc = Document::parse(text);
    let (stripped, kept) = doc.without_managed();
    let html = markdown::to_html_with_sourcepos(&stripped);

    let mut atoms: Vec<(Atom, Option<usize>)> = Vec::new();
    let mut line: Option<usize> = None;
    let mut code: Option<String> = None;
    let mut code_depth = 0usize;
    let mut alert_title = false;
    let mut join_next_block = false;
    let push = |atoms: &mut Vec<(Atom, Option<usize>)>, atom: Atom, line: Option<usize>, join: &mut bool| {
        if atom == Atom::Break {
            if *join {
                *join = false;
                return;
            }
            if atoms.last().is_none_or(|(a, _)| *a == Atom::Break) {
                return;
            }
        }
        atoms.push((atom, line));
    };

    for tag in tokenize(&html) {
        match &tag {
            Tag::Start { name, .. } => {
                if let Some(pos) = tag.attr("data-sourcepos") {
                    let start: Option<usize> = pos.split(':').next().and_then(|l| l.parse().ok());
                    line = start.and_then(|l| kept.get(l.checked_sub(1)?).map(|&i| i + 1));
                }
                let is_title = name == "p"
                    && tag
                        .attr("class")
                        .is_some_and(|c| c.contains("markdown-alert-title"));
                if is_title {
                    alert_title = true;
                } else if BLOCKS.contains(&name.as_str()) {
                    push(&mut atoms, Atom::Break, line, &mut join_next_block);
                }
                match name.as_str() {
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        push(&mut atoms, Atom::Heading(name.clone()), line, &mut false);
                    }
                    "a" => {
                        if let Some(href) = tag.attr("href").filter(|h| !h.is_empty()) {
                            push(&mut atoms, Atom::Link(href.to_owned()), line, &mut false);
                        }
                    }
                    "br" => push(&mut atoms, Atom::LineBreak, line, &mut false),
                    "img" => {
                        let src = tag.attr("src").unwrap_or_default().to_owned();
                        let alt = tag.attr("alt").unwrap_or_default().to_owned();
                        push(&mut atoms, Atom::Image { src, alt }, line, &mut false);
                    }
                    "code" => {
                        code_depth += 1;
                        code.get_or_insert_with(String::new);
                    }
                    _ => {}
                }
            }
            Tag::End(name) => {
                if name == "code" && code_depth > 0 {
                    code_depth -= 1;
                    if code_depth == 0
                        && let Some(text) = code.take()
                    {
                        push(&mut atoms, Atom::Code(text), line, &mut false);
                    }
                }
                if name == "p" && alert_title {
                    // The alert title stands in for a lead-in that shared its paragraph.
                    alert_title = false;
                    join_next_block = true;
                } else if BLOCKS.contains(&name.as_str()) {
                    push(&mut atoms, Atom::Break, line, &mut join_next_block);
                }
            }
            Tag::Text(text) => {
                if let Some(code) = code.as_mut() {
                    code.push_str(text);
                    continue;
                }
                let lowered = SHORTCODE.replace_all(&text.to_lowercase(), " ").into_owned();
                for word in WORD.find_iter(&lowered) {
                    // Variation selectors and joiners match \w but are not words.
                    if word.as_str().chars().any(char::is_alphanumeric) {
                        push(&mut atoms, Atom::Word(word.as_str().to_owned()), line, &mut false);
                    }
                }
            }
        }
    }
    while atoms.last().is_some_and(|(a, _)| *a == Atom::Break) {
        atoms.pop();
    }
    if atoms.first().is_some_and(|(a, _)| *a == Atom::Break) {
        atoms.remove(0);
    }
    atoms
}

fn show(atoms: &[(Atom, Option<usize>)]) -> String {
    const LIMIT: usize = 12;
    let mut out: Vec<String> = atoms.iter().take(LIMIT).map(|(a, _)| a.to_string()).collect();
    if atoms.len() > LIMIT {
        out.push("...".into());
    }
    out.join(" ")
}

/// Check that `after` says exactly what `before` says.
pub fn verify(before: &str, after: &str) -> Result<(), GuardViolation> {
    let a = atoms(before);
    let b = atoms(after);
    let same = |x: &[(Atom, Option<usize>)], y: &[(Atom, Option<usize>)]| {
        x.len() == y.len() && x.iter().zip(y).all(|((p, _), (q, _))| p == q)
    };
    if same(&a, &b) {
        return Ok(());
    }
    let prefix = a.iter().zip(&b).take_while(|((p, _), (q, _))| p == q).count();
    let suffix = a[prefix..]
        .iter()
        .rev()
        .zip(b[prefix..].iter().rev())
        .take_while(|((p, _), (q, _))| p == q)
        .count();
    let removed = &a[prefix..a.len() - suffix];
    let added = &b[prefix..b.len() - suffix];
    let line = removed
        .first()
        .or_else(|| a.get(prefix.saturating_sub(1)))
        .and_then(|(_, l)| *l);
    Err(GuardViolation {
        line,
        removed: show(removed),
        added: show(added),
    })
}

#[cfg(test)]
mod tests {
    use super::verify;

    #[test]
    fn identical_content_passes() {
        verify(
            "# T\n\nSome *text* with `code`.\n",
            "# T\n\nSome *text* with `code`.\n",
        )
        .expect("same");
    }

    #[test]
    fn catches_content_changes() {
        let cases = [
            ("# T\n\nkeep these words\n", "# T\n\nkeep words\n"),
            ("```\npip install x\n```\n", "```\npip install y\n```\n"),
            ("[docs](https://a)\n", "[docs](https://b)\n"),
            ("one\n\ntwo\n", "one\ntwo\n"),
            ("## Usage\n", "### Usage\n"),
            ("# T\n", "# T\n\nsurprise\n"),
            ("![logo](a.png)\n", "![logo](b.png)\n"),
        ];
        for (before, after) in cases {
            assert!(verify(before, after).is_err(), "{before:?} -> {after:?}");
        }
    }

    #[test]
    fn reports_where_and_what() {
        let error = verify("# T\n\nalpha\n\nbeta gamma\n", "# T\n\nalpha\n\nbeta\n").expect_err("dropped");
        assert_eq!(error.line, Some(5));
        assert_eq!(
            error.to_string(),
            "near line 5, the styled README would drop \"gamma\""
        );
    }

    #[test]
    fn alerts_stand_in_for_their_lead_in() {
        verify("> **Note:** read me\n", "> [!NOTE]\n> read me\n").expect("blockquote");
        verify("**Warning:** hot\n", "> [!WARNING]\n> hot\n").expect("paragraph");
        verify("> ⚠️ **Caution:** x\n", "> [!CAUTION]\n> x\n").expect("emoji");
        verify("> :warning: **Important:** x\n", "> [!IMPORTANT]\n> x\n").expect("shortcode");
    }

    #[test]
    fn managed_content_and_wrappers_are_ignored() {
        let after = "<div align=\"center\"><!-- readme-style:center -->\n\n# T\n\n\
                     <!-- readme-style:header theme=x -->\nbadges here\n<!-- /readme-style:header -->\n\n\
                     </div><!-- /readme-style:center -->\n";
        verify("# T\n", after).expect("generated content is not content");
    }
}
