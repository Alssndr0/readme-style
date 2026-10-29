//! The guarantees every theme keeps: content preserved, idempotent, switchable, faithful to
//! line endings. Checked on a fixture and on generated documents.

mod common;

use common::{content_lines, facts, run_with, theme};
use proptest::prelude::*;
use readme_style_core::theme::Themes;
use readme_style_core::{StyleError, style};

const README: &str = "# quickparse

A tiny parser for config files.

## Features

- Fast

## Install

```bash
pip install quickparse
```

> **Note:** needs Python 3.10.

## Usage

Use it.

## Changelog

- 1.0
- 0.9
- 0.8
- 0.7
- 0.6
- 0.5
- 0.4
- 0.3
- 0.2
- 0.1
- 0.0.9
- 0.0.8

## License

MIT
";

fn names() -> Vec<String> {
    Themes::builtin().names().into_iter().map(str::to_owned).collect()
}

#[test]
fn every_theme_is_idempotent() {
    for name in names() {
        let once = run_with(README, &theme(&name), &facts()).output;
        let twice = run_with(&once, &theme(&name), &facts()).output;
        assert_eq!(once, twice, "{name}");
        assert!(!run_with(&once, &theme(&name), &facts()).changed);
    }
}

#[test]
fn switching_themes_equals_styling_from_scratch() {
    for first in names() {
        for second in names() {
            let direct = run_with(README, &theme(&second), &facts()).output;
            let styled = run_with(README, &theme(&first), &facts()).output;
            let switched = run_with(&styled, &theme(&second), &facts()).output;
            assert_eq!(
                content_lines(&switched),
                content_lines(&direct),
                "{first} -> {second}"
            );
        }
    }
}

#[test]
fn line_endings_and_bom_survive() {
    let source = format!("\u{feff}{}", README.replace('\n', "\r\n"));
    let out = run_with(&source, &theme("startup"), &facts()).output;
    assert!(out.starts_with('\u{feff}'));
    assert!(
        !out.replace("\r\n", "").contains('\n'),
        "every newline stays CRLF"
    );
}

#[test]
fn a_missing_final_newline_stays_missing() {
    assert!(
        !run_with("# T\n\ntext", &theme("minimal"), &facts())
            .output
            .ends_with('\n')
    );
}

#[test]
fn already_styled_is_unchanged() {
    let once = run_with(README, &theme("startup"), &facts()).output;
    let outcome = run_with(&once, &theme("startup"), &facts());
    assert!(!outcome.changed && outcome.changed_by.is_empty());
}

/// Blocks a person writes: the constructs real READMEs use, including the lead-ins, sections
/// and HTML that transforms react to. `noise` is a character class for free-form lines.
fn user_block_with(noise: &'static str) -> impl Strategy<Value = String> {
    let word = "[a-zA-Z]{1,8}";
    let sentence = proptest::collection::vec(word, 1..8).prop_map(|w| w.join(" "));
    prop_oneof![
        sentence.clone().prop_map(|s| format!("{s}.")),
        (1u8..5, sentence.clone()).prop_map(|(level, s)| format!("{} {s}", "#".repeat(usize::from(level)))),
        sentence
            .clone()
            .prop_map(|s| format!("{s}\n{}", "=".repeat(s.len().max(3)))),
        sentence.clone().prop_map(|s| format!("{s}\n---")),
        Just("## Changelog".to_owned()),
        Just("## FAQ".to_owned()),
        Just("## 🚀 Quick Start".to_owned()),
        sentence.clone().prop_map(|s| format!("> **Note:** {s}")),
        sentence.clone().prop_map(|s| format!("> **Warning**\n> {s}")),
        sentence.clone().prop_map(|s| format!("**Warning:** {s}")),
        sentence.clone().prop_map(|s| format!("Tip: {s}")),
        sentence.clone().prop_map(|s| format!("> {s}\n>\n> {s}")),
        proptest::collection::vec(sentence.clone(), 1..4).prop_map(|items| items
            .iter()
            .map(|i| format!("- {i}"))
            .collect::<Vec<_>>()
            .join("\n")),
        Just("- [Intro](#intro)\n- [Usage](#usage)\n- [License](#license)".to_owned()),
        sentence
            .clone()
            .prop_map(|s| format!("```\n{s}\n<!-- readme-style:toc -->\n```")),
        sentence.clone().prop_map(|s| format!("    indented {s}")),
        Just("<p align=\"center\"><img src=\"logo.svg\"></p>".to_owned()),
        Just("<div align=\"center\">\n\n# Centered\n\n</div>".to_owned()),
        sentence
            .clone()
            .prop_map(|s| format!("<details>\n<summary>More</summary>\n\n{s}\n\n</details>")),
        Just("[![CI](https://img.shields.io/badge/ci-passing-green)](https://ci)".to_owned()),
        Just("![logo](assets/logo.svg)".to_owned()),
        Just("| a | b |\n|---|---|\n| 1 | 2 |".to_owned()),
        Just("---".to_owned()),
        sentence
            .clone()
            .prop_map(|s| format!("See [{s}](https://arxiv.org/abs/2401.00001)")),
        sentence
            .clone()
            .prop_map(|s| format!("A footnote[^1].\n\n[^1]: {s}")),
        sentence
            .clone()
            .prop_map(|s| format!("[{s}][ref]\n\n[ref]: https://example.com")),
        noise.prop_map(|n| n.trim().to_owned()),
    ]
}

/// Free-form lines without `<`: a tag that never closes swallows whatever follows it, in
/// GitHub's renderer as in a browser, so no change around it is safe and refusing is right.
fn user_block() -> impl Strategy<Value = String> {
    user_block_with("[a-zA-Z0-9 *_`#>\\[\\]()!:|=~.-]{1,40}")
}

/// Generated content as readme-style writes it, possibly left by another theme.
fn leftover_region() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("<!-- readme-style:header theme=minimal -->\nold badges\n<!-- /readme-style:header -->".to_owned()),
        Just("<!-- readme-style:toc theme=docs -->\n- [Old](#old)\n<!-- /readme-style:toc -->".to_owned()),
        Just("<!-- readme-style:logo theme=startup -->\n<img src=\"old.svg\">\n<!-- /readme-style:logo -->".to_owned()),
        Just("<!-- readme-style:back-to-top theme=docs -->\n<p align=\"right\"><a href=\"#x\">Back to top ↑</a></p>\n<!-- /readme-style:back-to-top -->".to_owned()),
    ]
}

/// Stray or mangled markers, as a careless edit leaves them.
fn stray_marker() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("<!-- readme-style:logo -->".to_owned()),
        Just("<!-- /readme-style:toc -->".to_owned()),
        Just("<div align=\"center\"><!-- readme-style:center -->".to_owned()),
        Just("</details><!-- /readme-style:collapse -->".to_owned()),
    ]
}

fn join(parts: &[(String, bool)], title: bool) -> String {
    let mut text = if title {
        "# Project\n\nA short pitch.\n\n".to_owned()
    } else {
        String::new()
    };
    for (i, (part, tight)) in parts.iter().enumerate() {
        if i > 0 {
            text.push_str(if *tight { "\n" } else { "\n\n" });
        }
        text.push_str(part);
    }
    text.push('\n');
    text
}

/// A README a person wrote, possibly styled before: user blocks, sometimes packed without blank
/// lines, and leftover regions with the blank lines readme-style puts around them.
fn well_formed() -> impl Strategy<Value = String> {
    let part = prop_oneof![
        4 => (user_block(), proptest::bool::ANY),
        1 => leftover_region().prop_map(|r| (r, false)),
    ];
    (proptest::collection::vec(part, 0..16), proptest::bool::ANY).prop_map(|(parts, title)| {
        // A region is always surrounded by blank lines, whatever its neighbour asked for.
        let mut fixed: Vec<(String, bool)> = Vec::new();
        for (part, tight) in parts {
            let is_region = part.starts_with("<!-- readme-style:");
            let previous_region = fixed
                .last()
                .is_some_and(|(p, _): &(String, bool)| p.starts_with("<!-- readme-style:"));
            fixed.push((part, tight && !is_region && !previous_region));
        }
        join(&fixed, title)
    })
}

/// Anything, including markers glued to content.
fn adversarial() -> impl Strategy<Value = String> {
    let part = prop_oneof![
        4 => user_block_with("[a-zA-Z0-9 *_`#>\\[\\]()<>!:|=~.-]{1,40}"),
        1 => leftover_region(),
        1 => stray_marker(),
    ];
    (
        proptest::collection::vec((part, proptest::bool::ANY), 0..16),
        proptest::bool::ANY,
    )
        .prop_map(|(parts, title)| join(&parts, title))
}

fn cases() -> u32 {
    std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300)
}

proptest! {
    #![proptest_config(ProptestConfig { cases: cases(), ..ProptestConfig::default() })]

    /// A well-formed README is always styled, never refused, and the result is stable.
    #[test]
    fn well_formed_readmes_keep_every_guarantee(source in well_formed(), index in 0usize..5) {
        let names = names();
        let name = &names[index % names.len()];
        let theme = theme(name);
        match style(&source, &theme, &facts()) {
            Ok(outcome) => {
                let again = style(&outcome.output, &theme, &facts()).expect("styled output styles again");
                prop_assert_eq!(&again.output, &outcome.output);
            }
            Err(StyleError::Guard(violation)) => prop_assert!(false, "{name}: {violation}\n{source}"),
            Err(StyleError::NotIdempotent(step)) => prop_assert!(false, "{name}: unstable in {step}\n{source}"),
            Err(error) => prop_assert!(false, "{name}: {error}\n{source}"),
        }
    }

    /// Line endings never change what is styled: any mix of LF, CRLF and lone CR gives the
    /// result of the LF file, with the endings put back.
    #[test]
    fn line_endings_do_not_change_the_result(
        source in well_formed(),
        endings in proptest::collection::vec(0..3usize, 64),
        index in 0usize..5,
    ) {
        let names = names();
        let name = &names[index % names.len()];
        let theme = theme(name);
        let mut mixed = String::new();
        for (i, line) in source.split_inclusive('\n').enumerate() {
            match line.strip_suffix('\n') {
                Some(body) => {
                    mixed.push_str(body);
                    let mut ending = ["\n", "\r\n", "\r"][endings[i % endings.len()]];
                    // CR, then an empty line ending in LF, would be one CRLF.
                    if body.is_empty() && ending == "\n" && mixed.ends_with('\r') {
                        ending = "\r";
                    }
                    mixed.push_str(ending);
                }
                None => mixed.push_str(line),
            }
        }
        let expected = style(&source, &theme, &facts()).expect("the LF file styles").output;
        let outcome = style(&mixed, &theme, &facts());
        prop_assert!(outcome.is_ok(), "{name}: {:?}\n{mixed:?}", outcome.err());
        let output = outcome.map(|o| o.output).unwrap_or_default();
        prop_assert_eq!(output.replace("\r\n", "\n").replace('\r', "\n"), expected);
        let again = style(&output, &theme, &facts()).expect("styles again");
        prop_assert!(!again.changed, "{name}: unstable\n{mixed:?}");
    }

    /// Mangled markers may be refused, but never crash, and what is written is stable.
    #[test]
    fn adversarial_input_never_panics_or_writes_unstable_output(source in adversarial(), index in 0usize..5) {
        let names = names();
        let theme = theme(&names[index % names.len()]);
        if let Ok(outcome) = style(&source, &theme, &facts()) {
            let again = style(&outcome.output, &theme, &facts());
            prop_assert!(again.is_ok_and(|a| a.output == outcome.output));
        }
    }
}
