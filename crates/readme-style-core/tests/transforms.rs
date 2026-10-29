//! Behavior of each transform, through the public API.

mod common;

use common::{facts, run, run_with, theme, theme_with};
use pretty_assertions::assert_eq;
use readme_style_core::project::{Logo, ProjectFacts};

#[test]
fn callouts_from_quotes_and_paragraphs() {
    let cases = [
        ("> **Note:** text", "> [!NOTE]\n> text"),
        ("> **Warning**: careful", "> [!WARNING]\n> careful"),
        ("> Tip: use it", "> [!TIP]\n> use it"),
        ("> ⚠️ **Caution:** hot", "> [!CAUTION]\n> hot"),
        ("> :warning: **Important:** read", "> [!IMPORTANT]\n> read"),
        (
            "**Note:** a paragraph\nover two lines",
            "> [!NOTE]\n> a paragraph\n> over two lines",
        ),
        ("> **Note**\n> on the next line", "> [!NOTE]\n> on the next line"),
    ];
    for (source, expected) in cases {
        assert_eq!(
            run(&format!("{source}\n"), "minimal"),
            format!("{expected}\n"),
            "{source}"
        );
    }
}

#[test]
fn callouts_leave_other_text_alone() {
    let untouched = [
        "> [!NOTE]\n> already an alert",
        "> **Note that** this is prose",
        "Note that this is prose.",
        "> **Note:**",
        "> **Note:**\n>\n> a separate paragraph",
        "- **Note:** inside a list",
        "> > **Note:** nested",
    ];
    for source in untouched {
        assert_eq!(
            run(&format!("{source}\n"), "minimal"),
            format!("{source}\n"),
            "{source}"
        );
    }
    let off = theme_with("minimal", "[callouts]\nenabled = false");
    assert_eq!(
        run_with("> **Note:** x\n", &off, &ProjectFacts::default()).output,
        "> **Note:** x\n"
    );
}

#[test]
fn long_changelogs_fold_and_keep_their_heading() {
    let body: Vec<String> = (0..12).map(|i| format!("- change {i}")).collect();
    let out = run(
        &format!(
            "# T\n\n## Changelog\n\n{}\n\n## License\n\nMIT\n",
            body.join("\n")
        ),
        "startup",
    );
    assert!(out.contains("## Changelog\n\n<details><summary>Click to expand</summary><!-- readme-style:collapse -->\n\n- change 0"), "{out}");
    assert!(
        out.contains("- change 11\n\n</details><!-- /readme-style:collapse -->\n\n## License"),
        "{out}"
    );
    assert!(!run("# T\n\n## Changelog\n\n- one\n- two\n", "startup").contains("<details>"));
}

#[test]
fn table_of_contents_lists_sections_with_anchors() {
    let readme =
        "# T\n\nIntro.\n\n## Install\n\n### From source\n\n## Usage\n\n## FAQ\n\n### Why?\n\n## License\n";
    let out = run(readme, "minimal");
    let expected = "<!-- readme-style:toc theme=minimal -->\n**Contents**\n\n- [Install](#install)\n  - [From source](#from-source)\n\
                    - [Usage](#usage)\n- [FAQ](#faq)\n- [License](#license)\n<!-- /readme-style:toc -->\n\n## Install";
    assert!(out.contains(expected), "{out}");
}

#[test]
fn table_of_contents_needs_sections_and_respects_the_users_own() {
    assert!(!run("# T\n\n## A\n\n## B\n", "minimal").contains("readme-style:toc"));
    let own = "# T\n\n## Table of Contents\n\n- [A](#a)\n\n## A\n\n## B\n\n## C\n\n## D\n";
    assert!(!run(own, "minimal").contains("readme-style:toc"));
}

#[test]
fn table_of_contents_entries_keep_inline_code_and_drop_links() {
    let readme = "# T\n\n## The `run` command\n\n## [Docs](https://x)\n\n## C++ & Rust\n\n## 🚀 Fast\n";
    let out = run(readme, "minimal");
    for entry in [
        "- [The `run` command](#the-run-command)",
        "- [Docs](#docs)",
        "- [C++ & Rust](#c--rust)",
        "- [🚀 Fast](#-fast)",
    ] {
        assert!(out.contains(entry), "missing {entry} in\n{out}");
    }
}

#[test]
fn minimal_puts_badges_under_the_title() {
    let project = ProjectFacts {
        license: Some("MIT".into()),
        license_file: Some("LICENSE".into()),
        ..ProjectFacts::default()
    };
    let out = run_with("# T\n\nPitch.\n", &theme("minimal"), &project).output;
    assert_eq!(
        out,
        "# T\n\n<!-- readme-style:header theme=minimal -->\n\
         [![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)\n\
         <!-- /readme-style:header -->\n\nPitch.\n"
    );
}

#[test]
fn badges_under_a_table_of_contents_still_count() {
    let readme =
        "# T\n\n## Table of contents\n\n- [A](#a)\n\n[![x](https://img.shields.io/badge/a-b-c)](y)\n\n## A\n";
    assert!(
        !run_with(readme, &theme("startup"), &facts())
            .output
            .contains("license-MIT")
    );
}

#[test]
fn a_readme_with_its_own_nav_row_gets_no_second_one() {
    let readme = "# T\n\n[Install](#install) · [Usage](#usage)\n\n## Install\n\n## Usage\n";
    let out = run(readme, "startup");
    assert!(!out.contains("**[Install](#install)**"), "{out}");
}

#[test]
fn readmes_with_badges_keep_their_own() {
    let readme = "# T\n\n[![x](https://img.shields.io/badge/a-b-c)](y)\n\nPitch.\n";
    assert_eq!(run_with(readme, &theme("minimal"), &facts()).output, readme);
}

#[test]
fn startup_centers_the_hero() {
    let out = run_with(
        "# T\n\nPitch.\n\n## Install\n\n## Usage\n",
        &theme("startup"),
        &facts(),
    )
    .output;
    let head = "<div align=\"center\"><!-- readme-style:center -->\n\n<!-- readme-style:logo theme=startup -->\n<picture>\n  \
                <source media=\"(prefers-color-scheme: dark)\" srcset=\"assets/logo-dark.svg\">\n  \
                <img src=\"assets/logo.svg\" alt=\"quickparse logo\" width=\"120\">\n</picture>\n<!-- /readme-style:logo -->\n\n\
                # T\n\nPitch.\n\n<!-- readme-style:header theme=startup -->\n";
    assert!(out.starts_with(head), "{out}");
    assert!(out.contains("**[Install](#install)** · **[Usage](#usage)**\n\n---\n<!-- /readme-style:header -->\n\n</div><!-- /readme-style:center -->"), "{out}");
}

#[test]
fn cli_theme_uses_key_caps_and_a_monochrome_palette() {
    let out = run_with("# tool\n\n## Install\n\n## Usage\n", &theme("cli"), &facts()).output;
    assert!(
        out.contains("<kbd>[Install](#install)</kbd> <kbd>[Usage](#usage)</kbd>"),
        "{out}"
    );
    assert!(
        out.contains("license-MIT-3d444d?labelColor=1f2328&style=flat-square"),
        "{out}"
    );
}

#[test]
fn docs_theme_adds_back_to_top_after_long_sections() {
    let long: Vec<String> = (0..25).map(|i| format!("Line {i}.\n")).collect();
    let readme = format!(
        "# Handbook\n\n## Long\n\n{}\n## Short\n\nOne line.\n\n## Also\n\nTwo.\n",
        long.join("\n")
    );
    let out = run(&readme, "docs");
    let link = "<!-- readme-style:back-to-top theme=docs -->\n<p align=\"right\"><a href=\"#handbook\">Back to top ↑</a></p>\n<!-- /readme-style:back-to-top -->";
    assert_eq!(out.matches(link).count(), 1, "{out}");
    assert!(out.contains(&format!("Line 24.\n\n{link}\n\n## Short")), "{out}");
}

#[test]
fn metadata_description_becomes_a_tagline_only_when_missing() {
    let project = ProjectFacts {
        description: Some("Fast things for everyone.".into()),
        ..ProjectFacts::default()
    };
    assert!(
        run_with("# T\n\n## A\n\n## B\n", &theme("startup"), &project)
            .output
            .contains("Fast things for everyone.")
    );
    let out = run_with(
        "# T\n\nFast things for everyone, truly.\n\n## A\n",
        &theme("startup"),
        &project,
    )
    .output;
    assert_eq!(out.matches("Fast things").count(), 1);
}

#[test]
fn an_existing_logo_gets_a_dark_variant() {
    let project = ProjectFacts {
        logo: Some(Logo {
            light: "assets/logo.png".into(),
            dark: Some("assets/logo-dark.png".into()),
        }),
        ..ProjectFacts::default()
    };
    let out = run_with(
        "[![Logo](./assets/logo.png)](https://site)\n\n# T\n",
        &theme("minimal"),
        &project,
    )
    .output;
    assert!(
        out.starts_with(
            "<p>\n<a href=\"https://site\">\n<picture>\n  <source media=\"(prefers-color-scheme: dark)\" srcset=\"./assets/logo-dark.png\">\n  \
             <img src=\"./assets/logo.png\" alt=\"Logo\">\n</picture>\n</a>\n</p>\n\n# T"
        ),
        "{out}"
    );
    let html = "<p align=\"center\"><img src=\"assets/logo.png\" width=\"200\"></p>\n\n# T\n";
    let out = run_with(html, &theme("minimal"), &project).output;
    assert!(
        out.contains(
            "<picture><source media=\"(prefers-color-scheme: dark)\" srcset=\"assets/logo-dark.png\"><img"
        ),
        "{out}"
    );
}

#[test]
fn a_logo_after_a_link_definition_keeps_the_definition() {
    let project = ProjectFacts {
        logo: Some(Logo {
            light: "assets/logo.svg".into(),
            dark: Some("assets/logo-dark.svg".into()),
        }),
        ..ProjectFacts::default()
    };
    let readme = "[A][ref]\n\n[ref]: https://example.com\n![logo](assets/logo.svg)\n";
    assert_eq!(run_with(readme, &theme("minimal"), &project).output, readme);
}

#[test]
fn nav_prefers_well_known_sections() {
    let names = [
        "Intro",
        "Features",
        "Background",
        "Installation",
        "Design",
        "Usage",
        "Internals",
        "Ideas",
        "License",
    ];
    let readme: String = std::iter::once("# T\n\n".to_owned())
        .chain(names.iter().map(|n| format!("## {n}\n\n")))
        .collect();
    let out = run(&readme, "startup");
    let nav = out.lines().find(|l| l.starts_with("**[")).expect("a nav bar");
    assert_eq!(
        nav,
        "**[Intro](#intro)** · **[Features](#features)** · **[Background](#background)** · \
         **[Installation](#installation)** · **[Usage](#usage)** · **[License](#license)**"
    );
}

#[test]
fn a_readme_without_a_title_keeps_its_hero_first() {
    let project = ProjectFacts {
        license: Some("MIT".into()),
        ..ProjectFacts::default()
    };
    let readme = "<p align=\"center\"><img src=\"logo.png\"></p>\n\n<h3 align=\"center\">Fast.</h3>\n\nIntro text.\n\n## A\n";
    let out = run_with(readme, &theme("startup"), &project).output;
    let header = out.find("readme-style:header").expect("a header");
    assert!(
        out.find("Fast.").expect("subtitle") < header && header < out.find("Intro text.").expect("intro"),
        "{out}"
    );
}

#[test]
fn a_notice_above_the_title_does_not_hide_it() {
    let readme = "> [!WARNING]\n> No longer maintained.\n\n---\n\n# Tool\n\nPitch.\n\n## A\n";
    let out = run_with(readme, &theme("startup"), &facts()).output;
    assert!(out.starts_with("> [!WARNING]"), "{out}");
    assert!(
        out.find("readme-style:logo").expect("logo") > out.find("---").expect("rule"),
        "{out}"
    );
}

#[test]
fn the_theme_is_recorded_in_markers() {
    let out = run_with("# T\n", &theme("cli"), &facts()).output;
    assert_eq!(readme_style_core::recorded_theme(&out).as_deref(), Some("cli"));
}
