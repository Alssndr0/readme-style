//! GitHub heading anchors, following the github-slugger algorithm.

use std::collections::HashMap;

use unicode_general_category::{GeneralCategory as G, get_general_category};

/// The anchor GitHub gives a heading with this text content.
///
/// github-slugger lowercases the text, keeps letters, marks, decimal and letter numbers,
/// connector punctuation (the underscore), hyphens and spaces, drops everything else, and
/// turns spaces into hyphens.
pub(crate) fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|&c| c == '-' || c == ' ' || keep(c))
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

fn keep(c: char) -> bool {
    matches!(
        get_general_category(c),
        G::UppercaseLetter
            | G::LowercaseLetter
            | G::TitlecaseLetter
            | G::ModifierLetter
            | G::OtherLetter
            | G::NonspacingMark
            | G::SpacingMark
            | G::EnclosingMark
            | G::DecimalNumber
            | G::LetterNumber
            | G::ConnectorPunctuation
    )
}

/// Unique anchors for a sequence of headings, numbering repeats the way GitHub does.
#[derive(Debug, Default)]
pub(crate) struct Slugger {
    seen: HashMap<String, usize>,
}

impl Slugger {
    pub(crate) fn slug(&mut self, text: &str) -> String {
        let base = slugify(text);
        let mut result = base.clone();
        while self.seen.contains_key(&result) {
            let count = self.seen.entry(base.clone()).or_insert(0);
            *count += 1;
            result = format!("{base}-{count}");
        }
        self.seen.insert(result.clone(), 0);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::{Slugger, slugify};

    #[test]
    fn matches_github() {
        let cases = [
            ("Hello World", "hello-world"),
            ("API Reference (v2)", "api-reference-v2"),
            ("C++ & Rust", "c--rust"),
            ("🚀 Quick Start", "-quick-start"),
            ("What's new?", "whats-new"),
            ("snake_case_name", "snake_case_name"),
            ("1.2.3 Release", "123-release"),
            ("Ünïcödé", "ünïcödé"),
            ("sampling / inference", "sampling--inference"),
            ("Version ²", "version-"),
        ];
        for (text, slug) in cases {
            assert_eq!(slugify(text), slug, "{text}");
        }
    }

    #[test]
    fn numbers_repeats() {
        let mut slugger = Slugger::default();
        let slugs: Vec<_> = ["Usage", "Usage", "Usage-1", "Usage"]
            .iter()
            .map(|t| slugger.slug(t))
            .collect();
        assert_eq!(slugs, ["usage", "usage-1", "usage-1-1", "usage-2"]);
    }
}
