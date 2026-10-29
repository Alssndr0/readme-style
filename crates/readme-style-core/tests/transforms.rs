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
fn callout_text_keeps_its_meaning() {
    let cases = [
        (
            "**Note:** # of retries is configurable.",
            "> [!NOTE]\n> \\# of retries is configurable.",
        ),
        (
            "**Note:** 1. Run setup first.",
            "> [!NOTE]\n> 1\\. Run setup first.",
        ),
        ("> **Warning:** - do not", "> [!WARNING]\n> \\- do not"),
        ("**Note:**\n2. step", "> [!NOTE]\n> 2\\. step"),
        ("**Note:**\n    x", "> [!NOTE]\n> x"),
        ("> **Note**\n> 2. step", "> [!NOTE]\n> 2\\. step"),
        // GitHub reads the line after `[!NOTE]` as a continuation: `--` would make a heading.
        ("**Note:** --", "> [!NOTE]\n> \\--"),
        ("**Note:** :--", "> [!NOTE]\n> \\:--"),
        // ... and the guard's parser as a paragraph start, where this would be a definition.
        (
            "**Note:** [foo]: https://example.com",
            "> [!NOTE]\n> [foo]\\: https://example.com",
        ),
        ("**Note:** [^1]: see below", "> [!NOTE]\n> [^1]\\: see below"),
        ("**Note:** -|", "> [!NOTE]\n> \\-|"),
        // A block right after a lead-in alone stays that block, inside the alert.
        (
            "> **Warning:**\n> - The format changed.\n> - Migrate first.",
            "> [!WARNING]\n> - The format changed.\n> - Migrate first.",
        ),
        ("> **Note:**\n> # Heading", "> [!NOTE]\n> # Heading"),
        // Two trailing spaces are a line break, and stay one.
        ("**Note:** text  \nmore", "> [!NOTE]\n> text  \n> more"),
    ];
    for (source, expected) in cases {
        assert_eq!(
            run(&format!("{source}\n"), "minimal"),
            format!("{expected}\n"),
            "{source}"
        );
    }
    let html = "Note: <div>x</div> hello\n";
    assert_eq!(
        run(html, "minimal"),
        html,
        "raw HTML at the start of a line would change meaning"
    );
}

#[test]
fn a_reference_link_after_a_lead_in_stays_a_link() {
    let readme = "# Proj\n\n**Note:** [Windows]: run it under WSL.\n\n[Windows]: https://learn.microsoft.com/windows/wsl\n";
    let out = run(readme, "minimal");
    assert!(out.contains("> [Windows]\\: run it under WSL."), "{out}");
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
        // GitHub shows `[!NOTE]` as text when nothing visible follows it.
        "> **Note:**\n> <!-- TODO: link the migration guide -->",
        "> **Note:**\n> <p align=\"center\">Centered words.</p>",
        "> **Tip:**\n> <!-- keep in sync -->\n> Use the cache.",
        "See[^1].\n\n> **Note:**\n> [^1]: The footnote.",
        // The alert title would take the place of the line break.
        "**Note:**  \nafter a break",
        "> **Note:**  \n> after a break",
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
fn nothing_is_placed_between_a_paragraph_and_its_table() {
    // Above a table, `[ref]: ...` is text; alone, it would be a link definition.
    let readme = "# Project\n\nA short pitch.\n\n[A][ref]\n\n[ref]: https://example.com\n\n[ref]: https://example.com\n| a | b |\n|---|---|\n| 1 | 2 |\n";
    for theme in ["minimal", "startup", "ml-research", "cli", "docs"] {
        let out = run(readme, theme);
        assert!(
            out.contains("[ref]: https://example.com\n| a | b |"),
            "{theme}:\n{out}"
        );
    }
}

#[test]
fn emoji_in_titles_do_not_hide_what_they_say() {
    // ❤️ and ⚖️ end with a variation selector, which counts as part of a word.
    let names = [
        "❤️ Contributing",
        "⚖️ License",
        "🛠️ Installation",
        "Design",
        "Internals",
        "Ideas",
        "Plugins",
        "Ecosystem",
    ];
    let readme: String = std::iter::once("# T\n\n".to_owned())
        .chain(names.iter().map(|n| format!("## {n}\n\n")))
        .collect();
    let nav = run(&readme, "startup");
    let nav = nav.lines().find(|l| l.starts_with("**[")).expect("a nav bar");
    assert!(nav.contains("Installation") && nav.contains("Ecosystem"), "{nav}");
    assert!(!nav.contains("Contributing") && !nav.contains("License"), "{nav}");
    let own_toc = "# T\n\n## 🗂️ Table of Contents\n\n- [A](#a)\n- [B](#b)\n\n## A\n\n## B\n\n## C\n";
    assert!(!run(own_toc, "docs").contains("readme-style:toc"));
    // `ℹ` is a letter to Unicode, and GitHub shows `:rocket:` as an emoji.
    let names = [
        "ℹ️ Overview",
        ":rocket: Installation",
        "Design",
        "Internals",
        "Ideas",
        "Plugins",
        "Ecosystem",
    ];
    let readme: String = std::iter::once("# T\n\n".to_owned())
        .chain(names.iter().map(|n| format!("## {n}\n\n")))
        .collect();
    let out = run(&readme, "startup");
    let nav = out.lines().find(|l| l.starts_with("**[")).expect("a nav bar");
    assert!(nav.contains("Overview") && nav.contains("Installation"), "{nav}");
}

#[test]
fn an_open_details_block_is_not_part_of_the_hero() {
    let readme = "<div align=\"center\">\n  <img src=\"logo.png\" width=\"100\">\n  <h1>Proj</h1>\n  <p>A thing.</p>\n</div>\n\n<details open>\n<summary>Table of Contents</summary>\n\n- [Install](#install)\n- [Usage](#usage)\n\n</details>\n\n## Install\n\nRun it.\n\n## Usage\n\nUse it.\n";
    let out = run(readme, "startup");
    let header = out.find("readme-style:header").expect("a header");
    assert!(
        header < out.find("<details open>").expect("the details block"),
        "{out}"
    );
    assert!(
        !out.contains("**[Install](#install)**"),
        "the README has its own contents:\n{out}"
    );
}

#[test]
fn the_hero_stops_at_folded_sections_and_the_wrapper_nests() {
    // An empty `<details open>` or `<table>` opening is no media.
    let details = "<p align=\"center\"><img src=\"logo.png\" width=\"100\"></p>\n\n<p align=\"center\">A tool for things.</p>\n\n<details open>\n\n<summary>More</summary>\n\nText.\n\n</details>\n\n## Install\n\nRun it.\n";
    let licensed = ProjectFacts {
        license: Some("MIT".into()),
        ..ProjectFacts::default()
    };
    for name in ["minimal", "startup"] {
        let out = run_with(details, &theme(name), &licensed).output;
        let header = out.find("<!-- /readme-style:header").expect("a header");
        assert!(
            header < out.find("<details open>").expect("details"),
            "{name}:\n{out}"
        );
    }
    // A tagline in a `<div>` closed by a later block: the header follows the badges inside it,
    // and the centering wrapper closes after the README's own `</div>`, so the two nest.
    let div = "# Thing\n\n<div align=\"center\">\n  <strong>A tool that makes things.</strong>\n\n[![CI](https://img.shields.io/badge/ci-passing-green)](x)\n\n</div>\n\n## Install\n\nRun it.\n\n## Usage\n\nUse it.\n";
    let out = run(div, "startup");
    let badges = out.find("[![CI]").expect("the README's badges");
    let header = out.find("readme-style:header").expect("a header");
    let own_close = out.find("</div>\n").expect("the README's </div>");
    let wrapper_close = out
        .find("</div><!-- /readme-style:center -->")
        .expect("the wrapper");
    assert!(
        badges < header && header < own_close && own_close < wrapper_close,
        "{out}"
    );
}

#[test]
fn headings_without_text_get_no_entry() {
    let readme =
        "# T\n\n## ![Demo](demo.gif)\n\n## <img src=\"logo.png\" alt=\"Logo\">\n\n## A\n\n## B\n\n## C\n";
    for theme in ["docs", "startup"] {
        let out = run(readme, theme);
        assert!(!out.contains("[](#"), "{theme}:\n{out}");
        assert!(out.contains("[A](#a)"), "{theme}:\n{out}");
    }
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
fn a_logo_with_a_non_ascii_name_gets_its_dark_variant() {
    let project = ProjectFacts {
        logo: Some(Logo {
            light: "assets/logo-ü.svg".into(),
            dark: Some("assets/logo-ü-dark.svg".into()),
        }),
        ..ProjectFacts::default()
    };
    let out = run_with("![logo](assets/logo-ü.svg)\n\n# T\n", &theme("minimal"), &project).output;
    assert!(out.contains(r#"srcset="assets/logo-%C3%BC-dark.svg""#), "{out}");
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
         **[Installation](#installation)** · **[Design](#design)** · **[Usage](#usage)**"
    );
}

#[test]
fn nav_puts_how_to_use_it_before_the_closing_sections() {
    let names = [
        "Features",
        "Installation",
        "Configuration",
        "API",
        "Benchmarks",
        "FAQ",
        "Changelog",
        "Contributing",
        "License",
    ];
    let readme: String = std::iter::once("# T\n\n".to_owned())
        .chain(names.iter().map(|n| format!("## {n}\n\n")))
        .collect();
    let out = run(&readme, "startup");
    let nav = out.lines().find(|l| l.starts_with("**[")).expect("a nav bar");
    assert_eq!(
        nav,
        "**[Features](#features)** · **[Installation](#installation)** · **[Configuration](#configuration)** · \
         **[API](#api)** · **[Benchmarks](#benchmarks)** · **[FAQ](#faq)**"
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

#[test]
fn front_matter_stays_first_and_untouched() {
    let readme = "---\nlicense: mit\nlanguage: en\n---\n\n# Model\n\n## A\n\n## B\n\n## C\n\n## D\n";
    for name in ["minimal", "startup", "docs"] {
        let out = run_with(readme, &theme(name), &facts()).output;
        assert!(
            out.starts_with("---\nlicense: mit\nlanguage: en\n---\n"),
            "{name}: {out}"
        );
        assert!(
            !out.contains("license: mit language: en]"),
            "{name}: front matter is not a heading"
        );
    }
}
