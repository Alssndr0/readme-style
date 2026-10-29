//! A table of contents with GitHub anchors, before the first section.

use std::sync::LazyLock;

use regex::Regex;

use super::{Context, names, sync_region};
use crate::document::{Document, Heading};
use crate::edit::Editor;
use crate::outline::is_toc_title;
use crate::theme::TocStyle;

static IMAGE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"!\[[^\]]*\]\([^)]*\)").expect("valid regex"));
static LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[([^\]]*)\]\([^)]*\)").expect("valid regex"));
static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").expect("valid regex"));
/// Sections whose subheadings are entries (versions, dates, questions), not structure.
static ENTRY_LISTS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\W*(change ?log|changes|release notes|release history|history|news|updates|what'?s new|faq|frequently asked questions)\W*$")
        .expect("valid regex")
});
/// Past this many entries only top-level sections are listed.
const MAX_ENTRIES: usize = 40;

/// The heading's own Markdown, minus links and images, which cannot nest inside a link.
fn entry_text(heading: &Heading) -> String {
    let text = IMAGE.replace_all(&heading.source, "");
    let text = LINK.replace_all(&text, "$1");
    let text = TAG.replace_all(&text, "");
    let text = text.trim();
    if text.is_empty() {
        heading.text.clone()
    } else {
        text.to_owned()
    }
}

fn render(doc: &Document, cx: &Context<'_>) -> Vec<String> {
    let settings = &cx.theme.toc;
    let outline = doc.outline();
    let Some(level) = outline.section_level else {
        return Vec::new();
    };
    let deepest = level.saturating_add(settings.depth - 1);
    let mut entries: Vec<&Heading> = Vec::new();
    let mut inside_entry_list = false;
    for heading in outline.body_headings.iter().map(|&i| &doc.headings[i]) {
        if heading.level == level {
            inside_entry_list = ENTRY_LISTS.is_match(&heading.text);
        } else if inside_entry_list {
            continue;
        }
        if heading.level <= deepest && !is_toc_title(&heading.text) {
            entries.push(heading);
        }
    }
    if entries.iter().filter(|h| h.level == level).count() < settings.min_sections {
        return Vec::new();
    }
    if entries.len() > MAX_ENTRIES {
        entries.retain(|h| h.level == level);
    }
    let items = entries.iter().map(|h| {
        let indent = "  ".repeat(usize::from(h.level - level));
        format!("{indent}- [{}](#{})", entry_text(h), h.slug)
    });
    match settings.style {
        TocStyle::None => Vec::new(),
        TocStyle::List => [format!("**{}**", settings.title), String::new()]
            .into_iter()
            .chain(items)
            .collect(),
        TocStyle::Details => [
            "<details>".to_owned(),
            format!("<summary><b>{}</b></summary>", settings.title),
            String::new(),
        ]
        .into_iter()
        .chain(items)
        .chain([String::new(), "</details>".to_owned()])
        .collect(),
    }
}

pub(super) fn apply(doc: &Document, cx: &mut Context<'_>) -> Option<Vec<String>> {
    let content = if doc.outline().has_user_toc {
        Vec::new()
    } else {
        render(doc, cx)
    };
    let mut ed = Editor::new(&doc.lines);
    sync_region(
        &mut ed,
        doc,
        &cx.theme.name,
        names::TOC,
        content,
        doc.outline().intro_end,
    );
    ed.apply()
}
