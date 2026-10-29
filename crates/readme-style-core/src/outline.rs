//! The structure of a README as a reader sees it: title, tagline, hero, sections.
//!
//! Only user content counts. Everything inside regions and on wrapper lines is ignored, so
//! the outline of a styled README equals the outline of the original.

use std::sync::LazyLock;

use regex::Regex;

use crate::document::{Block, BlockKind, Document};
use crate::html::closes;
use crate::theme::whole_title;
use crate::urls;

/// Elements that wrap whatever comes before their end tag.
const WRAPPERS: &[&str] = &["div", "details", "table", "section", "center"];

static TOC_TITLE: LazyLock<Regex> =
    LazyLock::new(|| whole_title("table of contents|contents|toc").expect("valid regex"));
static CENTERED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)align\s*=\s*["']?center|<center"#).expect("valid regex"));

/// The longest text that still reads as a one-line pitch.
const TAGLINE_MAX_CHARS: usize = 200;
/// The longest HTML block that still counts as part of a hero (a subtitle, a link row).
const HERO_HTML_MAX_CHARS: usize = 300;

pub(crate) fn is_toc_title(text: &str) -> bool {
    TOC_TITLE.is_match(text)
}

#[derive(Debug, Clone)]
pub(crate) struct Title {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) text: String,
}

/// A link in user content, with the line of the block it sits in.
#[derive(Debug, Clone)]
pub(crate) struct Link {
    pub(crate) href: String,
    pub(crate) text: String,
    pub(crate) line: usize,
}

#[derive(Debug)]
pub(crate) struct Outline {
    pub(crate) title: Option<Title>,
    /// Indexes into `Document::headings` of user headings after the title.
    pub(crate) body_headings: Vec<usize>,
    pub(crate) section_level: Option<u8>,
    /// Indexes into `Document::headings` of the top-level sections.
    pub(crate) sections: Vec<usize>,
    /// Line of the first section heading: the intro ends here.
    pub(crate) intro_end: usize,
    /// Index into `Document::blocks` of a short pitch right after the title.
    pub(crate) tagline: Option<usize>,
    /// First line of the hero: the title and the logos or centered HTML right above it.
    pub(crate) hero_start: usize,
    /// End of the hero: after the title, its tagline and badge rows.
    pub(crate) hero_end: usize,
    /// End of the logos and centered HTML that open a README without a title.
    pub(crate) lead_end: usize,
    pub(crate) has_user_badges: bool,
    pub(crate) has_hero_image: bool,
    pub(crate) has_user_toc: bool,
    pub(crate) has_user_nav: bool,
    pub(crate) links: Vec<Link>,
    /// The README text without managed lines.
    pub(crate) user_text: String,
}

/// A paragraph or HTML block holding only images, links around them, or comments.
pub(crate) fn is_media(block: &Block) -> bool {
    match block.kind {
        BlockKind::Html => block.text.trim().is_empty(),
        BlockKind::Paragraph => !block.images.is_empty() && block.text.trim().is_empty(),
        _ => false,
    }
}

/// An HTML block that is only a comment: invisible, so layout looks through it.
pub(crate) fn is_comment(block: &Block) -> bool {
    let source = block.literal.trim();
    block.kind == BlockKind::Html
        && source.starts_with("<!--")
        && source.ends_with("-->")
        && block.text.trim().is_empty()
}

/// A media block whose images are all status badges.
pub(crate) fn is_badge_row(block: &Block) -> bool {
    is_media(block) && !block.images.is_empty() && block.images.iter().all(|i| urls::is_badge(&i.src))
}

/// A paragraph of links and badges only (a language switcher, a row of shields), where any text
/// outside the links is separators such as `|`, `·` or `•`.
pub(crate) fn is_link_row(block: &Block) -> bool {
    if block.kind != BlockKind::Paragraph || (block.links.is_empty() && block.images.is_empty()) {
        return false;
    }
    let mut rest = block.text.clone();
    for link in &block.links {
        if let Some(at) = rest.find(&link.text) {
            rest.replace_range(at..at + link.text.len(), "");
        }
    }
    !rest.chars().any(char::is_alphanumeric)
}

/// A short, self-contained HTML block, such as a centered subtitle or a row of links.
fn is_hero_html(block: &Block, centered: bool) -> bool {
    if block.kind != BlockKind::Html {
        return false;
    }
    // HTML that leaves a wrapper open (`<details open>` before a list) wraps what follows, and
    // a folded section, a table or a list is body content: none of them is a hero.
    let source = block.literal.to_ascii_lowercase();
    if !closes(&source, WRAPPERS)
        || ["<details", "<table", "<ul", "<ol"]
            .iter()
            .any(|tag| source.contains(tag))
        || block.text.trim().chars().count() > HERO_HTML_MAX_CHARS
    {
        return false;
    }
    !centered || CENTERED.is_match(&source)
}

impl Outline {
    pub(crate) fn new(doc: &Document) -> Self {
        let user: Vec<(usize, &Block)> = doc
            .blocks
            .iter()
            .enumerate()
            .filter(|(_, b)| !b.managed)
            .collect();
        let title = find_title(doc, &user);
        let after_title = title.as_ref().map_or(0, |t| t.end);

        let body_headings: Vec<usize> = doc
            .headings
            .iter()
            .enumerate()
            .filter(|(_, h)| !h.managed && h.start >= after_title)
            .map(|(i, _)| i)
            .collect();
        let section_level = body_headings.iter().map(|&i| doc.headings[i].level).min();
        let sections: Vec<usize> = body_headings
            .iter()
            .copied()
            .filter(|&i| Some(doc.headings[i].level) == section_level)
            .collect();
        let intro_end = sections
            .first()
            .map_or(doc.lines.len(), |&i| doc.headings[i].start);

        let intro: Vec<(usize, &Block)> = user
            .iter()
            .copied()
            .filter(|(_, b)| b.start >= after_title && b.start < intro_end)
            .collect();
        // A README without a title may open with a preamble (sponsors, a notice) before the image
        // that starts its hero; the hero starts at that image when it comes early.
        let preamble = if title.is_none() {
            // Comments are invisible, so they do not count among the first three blocks.
            intro
                .iter()
                .enumerate()
                .filter(|(_, (_, b))| !is_comment(b))
                .take(3)
                .find(|(_, (_, b))| is_media(b) && !b.images.is_empty())
                .map_or(0, |(i, _)| i)
        } else {
            0
        };
        let hero_blocks = &intro[preamble..];
        let tagline = find_tagline(hero_blocks);

        let is_tagline = |index: usize| tagline == Some(index);
        let mut hero_end = if preamble > 0 {
            hero_blocks[0].1.start
        } else {
            after_title
        };
        for &(index, block) in hero_blocks {
            if is_comment(block) {
                continue;
            }
            let fits = if title.is_some() {
                is_tagline(index) || is_badge_row(block) || is_link_row(block) || is_hero_html(block, true)
            } else {
                is_tagline(index) || is_media(block) || is_hero_html(block, false)
            };
            if !fits {
                break;
            }
            hero_end = block.end;
        }

        let lead_start = if preamble > 0 { hero_blocks[0].1.start } else { 0 };
        let mut lead_end = lead_start;
        for (_, block) in user.iter().filter(|(_, b)| b.start >= lead_start) {
            if is_comment(block) {
                continue;
            }
            if !(is_media(block) || is_hero_html(block, false)) {
                break;
            }
            lead_end = block.end;
        }

        let hero_start = match &title {
            Some(title) => {
                let mut start = title.start;
                for (_, block) in user.iter().rev().filter(|(_, b)| b.start < title.start) {
                    if is_comment(block) {
                        continue;
                    }
                    if !(is_media(block) || is_hero_html(block, false)) {
                        break;
                    }
                    start = block.start;
                }
                start
            }
            None if hero_end > 0 => hero_blocks.first().or(user.first()).map_or(0, |(_, b)| b.start),
            None => 0,
        };

        let top_images: Vec<&str> = user
            .iter()
            .take_while(|(_, b)| b.start < intro_end)
            .flat_map(|(_, b)| b.images.iter().map(|i| i.src.as_str()))
            .collect();
        // Badges count as the README's own up to its first real section: a table of contents
        // section often comes first, with the badge row under it.
        let badge_zone_end = sections
            .iter()
            .map(|&i| &doc.headings[i])
            .find(|h| !is_toc_title(&h.text))
            .map_or(doc.lines.len(), |h| h.start);
        let has_user_badges = user
            .iter()
            .take_while(|(_, b)| b.start < badge_zone_end)
            .flat_map(|(_, b)| &b.images)
            .any(|i| urls::is_badge(&i.src));
        // A row of links to the README's own sections in the intro is a nav bar already.
        let has_user_nav = intro_blocks_of(&user, intro_end).any(|b| {
            b.kind != BlockKind::List && b.links.iter().filter(|l| l.href.starts_with('#')).count() >= 2
        });

        let links = user
            .iter()
            .flat_map(|(_, b)| {
                b.links.iter().map(|l| Link {
                    href: l.href.clone(),
                    text: l.text.clone(),
                    line: b.start,
                })
            })
            .collect();

        let user_text = (0..doc.lines.len())
            .filter(|&i| !doc.is_managed(i))
            .map(|i| doc.lines[i].as_str())
            .collect::<Vec<_>>()
            .join("\n");

        Self {
            title,
            body_headings,
            section_level,
            sections,
            intro_end,
            tagline,
            hero_start,
            hero_end,
            lead_end,
            has_user_badges,
            has_user_nav,
            has_hero_image: top_images.iter().any(|src| !urls::is_badge(src)),
            has_user_toc: has_user_toc(doc, &user),
            links,
            user_text,
        }
    }
}

/// User blocks before `intro_end`.
fn intro_blocks_of<'a>(user: &'a [(usize, &'a Block)], intro_end: usize) -> impl Iterator<Item = &'a Block> {
    user.iter()
        .map(|(_, b)| *b)
        .take_while(move |b| b.start < intro_end)
}

/// The title: an H1, an HTML `<h1>`, or an opening heading that no other heading outranks
/// (a README that starts with `## name` and uses `###` for its sections). Logos, badges,
/// paragraphs and notices may come before it.
fn find_title(doc: &Document, user: &[(usize, &Block)]) -> Option<Title> {
    let mut only_media = true;
    for (_, block) in user {
        match block.kind {
            BlockKind::Heading { level } => {
                let heading = doc.heading_at(block.start)?;
                let outranked = doc
                    .headings
                    .iter()
                    .any(|h| !h.managed && h.start != heading.start && h.level <= level);
                let has_others = doc
                    .headings
                    .iter()
                    .any(|h| !h.managed && h.start != heading.start);
                return (level == 1 || (only_media && has_others && !outranked)).then(|| Title {
                    start: block.start,
                    end: block.end,
                    text: heading.text.clone(),
                });
            }
            BlockKind::Html if block.literal.to_ascii_lowercase().contains("<h1") => {
                return Some(Title {
                    start: block.start,
                    end: block.end,
                    text: block.text.trim().to_owned(),
                });
            }
            BlockKind::Paragraph | BlockKind::Html | BlockKind::ThematicBreak => {
                only_media &= block.kind == BlockKind::Html || is_media(block);
            }
            // A notice above the title ("This project is no longer maintained").
            BlockKind::BlockQuote | BlockKind::Alert => only_media = false,
            _ => return None,
        }
    }
    None
}

/// A short first paragraph (or HTML text block) after the title, skipping logos and badges.
fn find_tagline(intro: &[(usize, &Block)]) -> Option<usize> {
    for &(index, block) in intro {
        if is_media(block) {
            continue;
        }
        let text = block.text.trim();
        let lines = match block.kind {
            BlockKind::Paragraph => block.end - block.start,
            // HTML that leaves an element open wraps what follows; it is no tagline.
            BlockKind::Html if block.images.is_empty() && closes(&block.literal, WRAPPERS) => {
                text.lines().count()
            }
            _ => return None,
        };
        let short = !text.is_empty() && text.chars().count() <= TAGLINE_MAX_CHARS && lines <= 2;
        return short.then_some(index);
    }
    None
}

fn has_user_toc(doc: &Document, user: &[(usize, &Block)]) -> bool {
    if doc.headings.iter().any(|h| !h.managed && is_toc_title(&h.text)) {
        return true;
    }
    user.iter().any(|(_, block)| match block.kind {
        // `<details><summary>Table of Contents</summary>` around the list.
        BlockKind::Html => is_toc_title(block.text.trim()),
        BlockKind::Paragraph => {
            matches!(
                block.text.trim().to_ascii_lowercase().as_str(),
                "[toc]" | "[[toc]]" | "[[_toc_]]"
            )
        }
        BlockKind::List => {
            let anchors = block.links.iter().filter(|l| l.href.starts_with('#')).count();
            block.items >= 3 && anchors * 5 >= block.items * 3
        }
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use crate::document::Document;

    #[test]
    fn title_tagline_and_sections() {
        let doc =
            Document::parse("# Name\n\nShort pitch.\n\nLonger intro.\n\n## Install\n\n### Pip\n\n## Usage\n");
        let outline = doc.outline();
        assert_eq!(outline.title.as_ref().map(|t| t.text.as_str()), Some("Name"));
        assert_eq!(outline.tagline.map(|i| doc.blocks[i].start), Some(2));
        let sections: Vec<_> = outline
            .sections
            .iter()
            .map(|&i| doc.headings[i].text.as_str())
            .collect();
        assert_eq!(sections, ["Install", "Usage"]);
        assert_eq!(outline.intro_end, 6);
        assert_eq!(outline.hero_end, 3);
    }

    #[test]
    fn opening_heading_is_the_title_when_nothing_outranks_it() {
        let doc = Document::parse("tool\n----\n\nintro\n\n### One\n\n### Two\n");
        assert_eq!(
            doc.outline().title.as_ref().map(|t| t.text.as_str()),
            Some("tool")
        );
        let doc = Document::parse("Intro text.\n\n## A\n");
        assert!(
            doc.outline().title.is_none(),
            "text before a lone heading means it is a section"
        );
    }

    #[test]
    fn html_titles_and_hero_images() {
        let doc = Document::parse(
            "<p align=\"center\"><img src=\"logo.png\"></p>\n<h1 align=\"center\">Name</h1>\n\ntext\n",
        );
        let outline = doc.outline();
        assert_eq!(outline.title.as_ref().map(|t| t.text.as_str()), Some("Name"));
        assert!(outline.has_hero_image);
        assert!(!outline.has_user_badges);
    }

    #[test]
    fn user_badges_and_toc() {
        let doc = Document::parse(
            "# T\n\n[![CI](https://github.com/o/r/actions/workflows/ci.yml/badge.svg)](x)\n\n\
             - [A](#a)\n- [B](#b)\n- [C](#c)\n\n## A\n## B\n## C\n",
        );
        let outline = doc.outline();
        assert!(outline.has_user_badges);
        assert!(outline.has_user_toc);
        assert_eq!(outline.hero_end, 3, "the badge row belongs to the hero");
    }

    #[test]
    fn a_preamble_before_the_hero_image_is_left_alone() {
        let doc = Document::parse(
            "<div align=\"center\">Special thanks to our sponsors, who keep this going; visit them all.</div>\n\n\
             <p align=\"center\"><img src=\"logo.png\"></p>\n\nA terminal UI for things.\n\nLong intro text here.\n\n## A\n",
        );
        let outline = doc.outline();
        assert_eq!(
            (outline.hero_start, outline.hero_end, outline.lead_end),
            (2, 5, 3)
        );
    }

    #[test]
    fn badges_and_language_links_belong_to_the_hero() {
        let doc = Document::parse(
            "# fd\n\n[![CI](https://img.shields.io/badge/a-b-c)](x)\n[中文](a.md) | [한국어](b.md)\n\nfd finds files.\nIt is fast and has many options that make it pleasant.\n\n## A\n",
        );
        assert_eq!(doc.outline().hero_end, 4);
    }

    #[test]
    fn hero_of_a_readme_without_title() {
        let doc = Document::parse(
            "<p align=\"center\"><img src=\"logo.png\"></p>\n\n<h3 align=\"center\">Fast.</h3>\n\nIntro.\n\n## A\n",
        );
        let outline = doc.outline();
        assert!(outline.title.is_none());
        assert_eq!((outline.lead_end, outline.hero_end), (3, 3));
    }
}
