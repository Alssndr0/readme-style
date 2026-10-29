//! A parsed README: top-level blocks with line spans, headings with GitHub anchors, and the
//! regions and wrapper lines readme-style manages.
//!
//! Parsing uses comrak (a port of cmark-gfm, GitHub's own parser). Text, links and images are
//! read from the AST, where reference links are already resolved; raw HTML (blocks and inline
//! tags) is tokenized, so `<a href>` and `<img>` count the same as their Markdown forms. The
//! comrak arena is dropped at the end of [`Document::parse`]; everything kept is owned.

use std::cell::OnceCell;
use std::sync::LazyLock;

use regex::Regex;

use comrak::Arena;
use comrak::nodes::{AstNode, NodeValue};

use crate::Diagnostic;
use crate::html::{self, ImageRef, LinkRef, Tag};
use crate::markdown;
use crate::markers::{self, Region};
use crate::outline::Outline;
use crate::slug::Slugger;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockKind {
    Heading { level: u8 },
    Paragraph,
    Html,
    Code { fenced: bool },
    List,
    BlockQuote,
    Alert,
    Table,
    ThematicBreak,
    Footnote,
    Other,
}

/// A paragraph made of exactly one image, maybe wrapped in a link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SoleImage {
    pub(crate) src: String,
    pub(crate) alt: String,
    pub(crate) title: Option<String>,
    pub(crate) href: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct Block {
    pub(crate) kind: BlockKind,
    /// First line (0-based).
    pub(crate) start: usize,
    /// One past the last line.
    pub(crate) end: usize,
    /// Starts inside a region or on a wrapper line.
    pub(crate) managed: bool,
    /// The raw source of an HTML block, empty for other kinds.
    pub(crate) literal: String,
    /// Text content as a browser shows it (image alt text excluded).
    pub(crate) text: String,
    pub(crate) images: Vec<ImageRef>,
    pub(crate) links: Vec<LinkRef>,
    /// Items of a list.
    pub(crate) items: usize,
    pub(crate) sole_image: Option<SoleImage>,
}

#[derive(Debug, Clone)]
pub(crate) struct Heading {
    pub(crate) level: u8,
    pub(crate) start: usize,
    pub(crate) end: usize,
    /// Text content, as GitHub uses for the anchor.
    pub(crate) text: String,
    /// The heading as Markdown fit for link text: text escaped, code spans kept, links reduced
    /// to their text, images and raw HTML dropped.
    pub(crate) label: String,
    pub(crate) slug: String,
    pub(crate) managed: bool,
}

impl Heading {
    /// Whether a link can name and reach it: a heading made only of an image or HTML has no
    /// label, and no anchor of its own.
    pub(crate) fn has_anchor_text(&self) -> bool {
        !self.label.is_empty() && !self.slug.is_empty()
    }
}

#[derive(Debug)]
pub(crate) struct Document {
    pub(crate) lines: Vec<String>,
    final_newline: bool,
    pub(crate) blocks: Vec<Block>,
    /// Top-level headings. Their anchors account for every heading in the document.
    pub(crate) headings: Vec<Heading>,
    pub(crate) regions: Vec<Region>,
    pub(crate) wrapper_lines: Vec<usize>,
    managed: Vec<bool>,
    pub(crate) diagnostics: Vec<Diagnostic>,
    /// First line of a fenced code block that is never closed: everything after it is code, so
    /// nothing may be placed after it.
    pub(crate) open_fence: Option<usize>,
    outline: OnceCell<Outline>,
}

fn lines_of<'a>(node: &'a AstNode<'a>) -> (usize, usize) {
    let pos = node.data.borrow().sourcepos;
    let start = pos.start.line.saturating_sub(1);
    (start, pos.end.line.max(start + 1))
}

/// Text content (as a browser shows it), images and links of a subtree.
#[derive(Default)]
struct Collector {
    out: html::Contents,
    /// Open links: target and text so far.
    open: Vec<(String, String)>,
    skip: usize,
}

impl Collector {
    fn of<'a>(node: &'a AstNode<'a>) -> html::Contents {
        let mut collector = Collector::default();
        collector.walk(node);
        for (href, text) in collector.open.drain(..).rev() {
            if !href.is_empty() {
                collector.out.links.push(LinkRef {
                    href,
                    text: text.trim().to_owned(),
                });
            }
        }
        collector.out
    }

    fn text(&mut self, text: &str) {
        if self.skip > 0 {
            return;
        }
        self.out.text.push_str(text);
        if let Some((_, link_text)) = self.open.last_mut() {
            link_text.push_str(text);
        }
    }

    fn close_link(&mut self) {
        if let Some((href, text)) = self.open.pop()
            && !href.is_empty()
        {
            self.out.links.push(LinkRef {
                href,
                text: text.trim().to_owned(),
            });
        }
    }

    fn image(&mut self, src: String, alt: String) {
        if let Some((_, link_text)) = self.open.last_mut() {
            link_text.push_str(&alt);
        }
        if !src.is_empty() {
            self.out.images.push(ImageRef { src, alt });
        }
    }

    fn walk<'a>(&mut self, node: &'a AstNode<'a>) {
        match &node.data.borrow().value {
            NodeValue::Text(text) => self.text(text),
            NodeValue::Code(code) => self.text(&code.literal),
            NodeValue::SoftBreak | NodeValue::LineBreak => self.text("\n"),
            NodeValue::Image(link) => {
                let alt = Collector::of_children(node).text;
                self.image(link.url.clone(), alt);
                return;
            }
            NodeValue::Link(link) => {
                self.open.push((link.url.clone(), String::new()));
                for child in node.children() {
                    self.walk(child);
                }
                self.close_link();
                return;
            }
            NodeValue::HtmlInline(raw) => self.html(raw),
            NodeValue::HtmlBlock(block) => self.html(&block.literal),
            NodeValue::FootnoteReference(_) => return,
            _ => {}
        }
        for child in node.children() {
            self.walk(child);
        }
    }

    fn of_children<'a>(node: &'a AstNode<'a>) -> html::Contents {
        let mut collector = Collector::default();
        for child in node.children() {
            collector.walk(child);
        }
        collector.out
    }

    fn html(&mut self, raw: &str) {
        for tag in html::tokenize(raw) {
            match &tag {
                Tag::Start { name, .. } if name == "script" || name == "style" => self.skip += 1,
                Tag::End(name) if (name == "script" || name == "style") && self.skip > 0 => self.skip -= 1,
                Tag::Start { name, .. } if name == "a" => {
                    self.open
                        .push((tag.attr("href").unwrap_or_default().to_owned(), String::new()));
                }
                Tag::End(name) if name == "a" => self.close_link(),
                Tag::Start { name, .. } if name == "img" => {
                    let src = tag.attr("src").unwrap_or_default().to_owned();
                    let alt = tag.attr("alt").unwrap_or_default().to_owned();
                    self.image(src, alt);
                }
                Tag::Text(text) => self.text(text),
                _ => {}
            }
        }
    }
}

/// A table directly under a paragraph (no blank line between) is parsed together with it, and
/// the paragraph's lines can read differently on their own: before a table, `[x]: url` is text,
/// alone it is a link definition. The two are one block, so nothing is placed between them.
fn join_tables(blocks: Vec<Block>) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::with_capacity(blocks.len());
    for block in blocks {
        if let Some(previous) = out.last_mut()
            && previous.kind == BlockKind::Paragraph
            && block.kind == BlockKind::Table
            && previous.end == block.start
        {
            previous.kind = BlockKind::Table;
            previous.end = block.end;
            previous.text = format!("{} {}", previous.text, block.text);
            previous.images.extend(block.images);
            previous.links.extend(block.links);
            previous.sole_image = None;
            continue;
        }
        out.push(block);
    }
    out
}

/// Inline content as Markdown that can sit inside `[...]`: text is escaped where it could
/// change meaning, code spans keep their backticks, links become their text, and images, raw
/// HTML and footnote marks are dropped.
fn link_label<'a>(node: &'a AstNode<'a>) -> String {
    fn walk<'a>(node: &'a AstNode<'a>, out: &mut String) {
        for child in node.children() {
            match &child.data.borrow().value {
                NodeValue::Text(text) => escape_label_text(text, out),
                NodeValue::Code(code) => out.push_str(&code_span(&code.literal)),
                NodeValue::SoftBreak | NodeValue::LineBreak => out.push(' '),
                NodeValue::Image(_) | NodeValue::HtmlInline(_) | NodeValue::FootnoteReference(_) => {}
                _ => walk(child, out),
            }
        }
    }
    let mut out = String::new();
    walk(node, &mut out);
    out.trim().to_owned()
}

fn escape_label_text(text: &str, out: &mut String) {
    let chars: Vec<char> = text.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        let before = i.checked_sub(1).and_then(|j| chars.get(j)).copied();
        let after = chars.get(i + 1).copied();
        let word = |x: Option<char>| x.is_some_and(char::is_alphanumeric);
        let escape = match c {
            '\\' | '`' | '*' | '[' | ']' | '<' | '~' => true,
            // Inside a word an underscore cannot start emphasis.
            '_' => !(word(before) && word(after)),
            // Only an entity-like `&name;` or `&#..;` could be read as one.
            '&' => after.is_some_and(|a| a.is_ascii_alphanumeric() || a == '#'),
            _ => false,
        };
        if escape {
            out.push('\\');
        }
        out.push(c);
    }
}

/// A code span for `code`, fenced with more backticks than any run inside it.
fn code_span(code: &str) -> String {
    let longest = code.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(longest + 1);
    let pad = if code.starts_with('`') || code.ends_with('`') {
        " "
    } else {
        ""
    };
    format!("{fence}{pad}{code}{pad}{fence}")
}

/// A paragraph made only of one image, or one link around one image.
fn sole_image<'a>(paragraph: &'a AstNode<'a>) -> Option<SoleImage> {
    let significant = |node: &'a AstNode<'a>| -> Vec<&'a AstNode<'a>> {
        node.children()
            .filter(|c| match &c.data.borrow().value {
                NodeValue::Text(t) => !t.trim().is_empty(),
                NodeValue::SoftBreak => false,
                _ => true,
            })
            .collect()
    };
    let children = significant(paragraph);
    let [only] = children.as_slice() else { return None };
    let (image, href) = match &only.data.borrow().value {
        NodeValue::Image(_) => (*only, None),
        NodeValue::Link(link) => {
            let inner = significant(only);
            let [image] = inner.as_slice() else { return None };
            if !matches!(image.data.borrow().value, NodeValue::Image(_)) {
                return None;
            }
            (*image, Some(link.url.clone()))
        }
        _ => return None,
    };
    let NodeValue::Image(link) = &image.data.borrow().value else {
        return None;
    };
    Some(SoleImage {
        src: link.url.clone(),
        alt: Collector::of_children(image).text,
        title: (!link.title.is_empty()).then(|| link.title.clone()),
        href,
    })
}

impl Document {
    pub(crate) fn parse(text: &str) -> Self {
        let final_newline = text.ends_with('\n');
        let body = text.strip_suffix('\n').unwrap_or(text);
        let lines: Vec<String> = if text.is_empty() {
            Vec::new()
        } else {
            body.split('\n').map(str::to_owned).collect()
        };
        let count = lines.len();

        let options = markdown::options();
        let arena = Arena::new();
        let root = comrak::parse_document(&arena, text, &options);

        let mut in_code = vec![false; count];
        let mut open_fence: Option<usize> = None;
        for node in root.descendants() {
            if let NodeValue::CodeBlock(code) = &node.data.borrow().value {
                let (start, end) = lines_of(node);
                for flag in in_code.iter_mut().take(end.min(count)).skip(start) {
                    *flag = true;
                }
                let top_level = node.parent().is_some_and(|p| p.same_node(root));
                if code.fenced && !code.closed && top_level {
                    open_fence = Some(open_fence.map_or(start, |f| f.min(start)));
                }
            }
        }

        let scan = markers::scan(&lines, &in_code);
        let mut managed = vec![false; count];
        for region in &scan.regions {
            for flag in managed
                .iter_mut()
                .take(region.end_exclusive().min(count))
                .skip(region.start)
            {
                *flag = true;
            }
        }
        for &line in &scan.wrappers {
            managed[line] = true;
        }
        let is_managed = |line: usize| managed.get(line).copied().unwrap_or(false);

        let mut slugger = Slugger::default();
        let mut headings = Vec::new();
        for node in root.descendants() {
            let level = match &node.data.borrow().value {
                NodeValue::Heading(heading) => heading.level,
                _ => continue,
            };
            let text = Collector::of(node).text.trim().to_owned();
            let slug = slugger.slug(&text);
            let top_level = node.parent().is_some_and(|p| p.same_node(root));
            if top_level {
                let (start, end) = lines_of(node);
                headings.push(Heading {
                    level,
                    start,
                    end,
                    label: link_label(node),
                    text,
                    slug,
                    managed: is_managed(start),
                });
            }
        }

        let blocks = root
            .children()
            .map(|node| {
                let (start, end) = lines_of(node);
                let contents = Collector::of(node);
                let (kind, literal, items) = match &node.data.borrow().value {
                    NodeValue::Heading(h) => (BlockKind::Heading { level: h.level }, String::new(), 0),
                    NodeValue::Paragraph => (BlockKind::Paragraph, String::new(), 0),
                    NodeValue::HtmlBlock(h) => (BlockKind::Html, h.literal.clone(), 0),
                    NodeValue::CodeBlock(c) => (BlockKind::Code { fenced: c.fenced }, String::new(), 0),
                    NodeValue::List(_) => (BlockKind::List, String::new(), node.children().count()),
                    NodeValue::BlockQuote => (BlockKind::BlockQuote, String::new(), 0),
                    NodeValue::Alert(_) => (BlockKind::Alert, String::new(), 0),
                    NodeValue::Table(_) => (BlockKind::Table, String::new(), 0),
                    NodeValue::ThematicBreak => (BlockKind::ThematicBreak, String::new(), 0),
                    NodeValue::FootnoteDefinition(_) => (BlockKind::Footnote, String::new(), 0),
                    _ => (BlockKind::Other, String::new(), 0),
                };
                let sole_image = if kind == BlockKind::Paragraph {
                    sole_image(node)
                } else {
                    None
                };
                let end = end.min(count.max(start + 1));
                let managed_here = !is_managed(start) && (start..end).any(is_managed);
                // Raw HTML without a blank line after it swallows the lines that follow, even
                // generated ones. Those are not the user's content.
                let (literal, contents) = if kind == BlockKind::Html && managed_here {
                    let own: Vec<&str> = (start..end)
                        .filter(|&i| !is_managed(i))
                        .map(|i| lines[i].as_str())
                        .collect();
                    let own = own.join("\n");
                    let mut collector = Collector::default();
                    collector.html(&own);
                    (own, collector.out)
                } else {
                    (literal, contents)
                };
                Block {
                    kind,
                    start,
                    end,
                    managed: is_managed(start),
                    literal,
                    text: contents.text,
                    images: contents.images,
                    links: contents.links,
                    items,
                    sole_image,
                }
            })
            .collect::<Vec<_>>();
        // comrak moves footnote definitions to the end of the tree; keep source order.
        let mut blocks = blocks;
        blocks.sort_by_key(|b| b.start);
        let blocks = join_tables(blocks);

        Self {
            lines,
            final_newline,
            blocks,
            headings,
            regions: scan.regions,
            wrapper_lines: scan.wrappers,
            managed,
            diagnostics: scan.diagnostics,
            open_fence,
            outline: OnceCell::new(),
        }
    }

    /// Join edited lines back into text, keeping the original final newline (or its absence).
    pub(crate) fn render(&self, lines: &[String]) -> String {
        if lines.is_empty() {
            return String::new();
        }
        let mut out = lines.join("\n");
        if self.final_newline {
            out.push('\n');
        }
        out
    }

    pub(crate) fn is_managed(&self, line: usize) -> bool {
        self.managed.get(line).copied().unwrap_or(false)
    }

    pub(crate) fn region(&self, name: &str) -> Option<&Region> {
        self.regions.iter().find(|r| r.name == name)
    }

    pub(crate) fn heading_at(&self, line: usize) -> Option<&Heading> {
        self.headings.iter().find(|h| h.start == line)
    }

    /// Blocks the user wrote, in document order.
    pub(crate) fn user_blocks(&self) -> impl Iterator<Item = &Block> {
        self.blocks.iter().filter(|b| !b.managed)
    }

    pub(crate) fn outline(&self) -> &Outline {
        self.outline.get_or_init(|| Outline::new(self))
    }

    /// Whether deleting lines `start..end` needs a separator to keep what follows in place.
    /// Blank lines end neither a list nor indented code, while the deleted marker comments did:
    /// without one, an indented line would join the list item, footnote or code block above, and
    /// a list would merge with the list above it.
    pub(crate) fn needs_separator(&self, start: usize, end: usize) -> bool {
        static LIST_ITEM: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^ {0,3}(?:[-*+]|\d{1,9}[.)])(?:[ \t]|$)").expect("valid regex"));
        let indented = |line: &str| line.starts_with('\t') || line.starts_with("    ");
        let Some(next) = (end..self.lines.len()).find(|&i| !self.lines[i].trim().is_empty()) else {
            return false;
        };
        if self.is_managed(next) {
            // More generated lines follow; they keep the blocks apart or decide when deleted.
            return false;
        }
        let next_indented = indented(&self.lines[next]);
        let next_is_item = LIST_ITEM.is_match(&self.lines[next]);
        if !next_indented && !next_is_item {
            return false;
        }
        let Some(above) = (0..start)
            .rev()
            .find(|&i| !self.lines[i].trim().is_empty() && !self.is_managed(i))
        else {
            return false;
        };
        // A line in no block is part of something the parser hides, such as a footnote nothing
        // references, which still takes indented lines after it: keep them apart.
        let Some(previous) = self.blocks.iter().find(|b| b.start <= above && above < b.end) else {
            return true;
        };
        match previous.kind {
            BlockKind::List => true,
            BlockKind::Footnote | BlockKind::Code { fenced: false } | BlockKind::Other => next_indented,
            _ => false,
        }
    }

    /// Whether a line shows nothing: blank, or only an HTML comment.
    pub(crate) fn is_invisible(&self, line: usize) -> bool {
        let text = self.lines[line].trim();
        text.is_empty()
            || text
                .strip_prefix("<!--")
                .and_then(|rest| rest.find("-->"))
                .is_some_and(|i| i + 3 == text.len() - 4)
    }

    /// Whether lines `start..=end` sit inside one block the user wrote, as when raw HTML without
    /// a blank line after it swallows the lines that follow. (A block opened by a generated line
    /// is not the user's, and stripping that line ends it.)
    pub(crate) fn inside_block(&self, start: usize, end: usize) -> bool {
        self.blocks
            .iter()
            .any(|b| !b.managed && b.start < start && b.end > end)
    }

    /// The README without anything readme-style manages, and for each remaining line its
    /// line number in the original.
    ///
    /// A run of managed lines between blocks becomes an empty comment, which separates blocks
    /// exactly as the markers did (a comment ends a paragraph, a list or indented code). A run
    /// inside a block is removed, leaving the block whole.
    pub(crate) fn without_managed(&self) -> (String, Vec<usize>) {
        let mut lines = Vec::with_capacity(self.lines.len());
        let mut origin = Vec::with_capacity(self.lines.len());
        let mut i = 0;
        while i < self.lines.len() {
            if !self.is_managed(i) {
                lines.push(self.lines[i].clone());
                origin.push(i);
                i += 1;
                continue;
            }
            let start = i;
            while i < self.lines.len() && self.is_managed(i) {
                i += 1;
            }
            if !self.inside_block(start, i - 1) {
                lines.push(crate::edit::SEPARATOR.to_owned());
                origin.push(start);
            }
        }
        (self.render(&lines), origin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_have_line_spans_and_kinds() {
        let doc = Document::parse("Title\n=====\n\n> [!NOTE]\n> hi\n\n- a\n- b\n\n```\ncode\n```\n");
        let kinds: Vec<_> = doc.blocks.iter().map(|b| (b.kind, b.start, b.end)).collect();
        assert_eq!(
            kinds,
            [
                (BlockKind::Heading { level: 1 }, 0, 2),
                (BlockKind::Alert, 3, 5),
                (BlockKind::List, 6, 8),
                (BlockKind::Code { fenced: true }, 9, 12),
            ]
        );
        assert_eq!(doc.blocks[2].items, 2);
    }

    #[test]
    fn headings_have_anchors_counted_across_the_document() {
        let doc = Document::parse("# Usage\n\n> ## Usage\n\n## Usage `x`\n");
        let slugs: Vec<_> = doc.headings.iter().map(|h| h.slug.as_str()).collect();
        assert_eq!(slugs, ["usage", "usage-x"]);
        assert_eq!(doc.headings[1].label, "Usage `x`");
    }

    #[test]
    fn labels_survive_code_html_and_links() {
        let doc = Document::parse(
            "## `Vec<T>` helpers\n\n## The `<Button>` component\n\n## [Docs](https://x) and <b>bold</b>\n\n## Uses `` ` `` ticks & a_b\n",
        );
        let labels: Vec<_> = doc.headings.iter().map(|h| h.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "`Vec<T>` helpers",
                "The `<Button>` component",
                "Docs and bold",
                "Uses `` ` `` ticks & a_b"
            ]
        );
    }

    #[test]
    fn markers_in_code_are_not_managed() {
        let doc = Document::parse("```\n<!-- readme-style:toc -->\n<!-- /readme-style:toc -->\n```\n");
        assert!(doc.regions.is_empty());
        assert!(!doc.is_managed(1));
    }

    #[test]
    fn links_and_images_come_from_markdown_and_html() {
        let doc = Document::parse(
            "[![CI](https://img.shields.io/x)](https://ci)\n\n<p><a href=\"https://docs\">Docs</a></p>\n\n[ref][r]\n\n[r]: https://ref\n",
        );
        assert_eq!(doc.blocks[0].images[0].src, "https://img.shields.io/x");
        assert_eq!(
            doc.blocks[0].sole_image.as_ref().and_then(|s| s.href.as_deref()),
            Some("https://ci")
        );
        assert_eq!(doc.blocks[1].links[0].href, "https://docs");
        assert_eq!(doc.blocks[2].links[0].href, "https://ref");
    }

    #[test]
    fn collects_text_links_and_images_from_markdown_and_html() {
        let doc = Document::parse(
            "See [the **docs**](https://x) and <a href=\"y\"><img src=\"b.svg\" alt=\"CI\"></a> &amp; more\n",
        );
        let block = &doc.blocks[0];
        assert_eq!(block.text, "See the docs and  & more");
        let links: Vec<_> = block
            .links
            .iter()
            .map(|l| (l.href.as_str(), l.text.as_str()))
            .collect();
        assert_eq!(links, [("https://x", "the docs"), ("y", "CI")]);
        assert_eq!(
            block.images,
            [ImageRef {
                src: "b.svg".into(),
                alt: "CI".into()
            }]
        );
    }

    #[test]
    fn html_comments_and_scripts_are_not_text() {
        let doc = Document::parse("<!-- hidden -->\n<p>shown</p><script>var x = 1;</script>\n");
        assert_eq!(
            doc.blocks[0].text.trim(),
            "",
            "a comment on its own line is a block of its own"
        );
        assert_eq!(doc.blocks[1].text.trim(), "shown");
    }

    #[test]
    fn blocks_are_in_source_order_even_with_footnotes() {
        let doc = Document::parse("A note[^1].\n\n[^1]: Text.\n\n## After\n");
        let starts: Vec<usize> = doc.blocks.iter().map(|b| b.start).collect();
        assert_eq!(starts, [0, 2, 4]);
        assert_eq!(doc.blocks[1].kind, BlockKind::Footnote);
    }

    #[test]
    fn separators_keep_lists_and_indented_code_in_place() {
        let doc = Document::parse("- a\n\nX\n\n- b\n\npara\n\nY\n\n- c\n");
        assert!(doc.needs_separator(2, 3), "a list after a list would merge");
        assert!(
            !doc.needs_separator(7, 8),
            "a list after a paragraph starts fresh"
        );
        let doc = Document::parse("para\n\nX\n\n    code\n");
        assert!(
            !doc.needs_separator(2, 3),
            "indented code after a paragraph stays code"
        );
    }

    #[test]
    fn invisible_lines() {
        let doc = Document::parse("\n<!-- -->\n<!---->\n<!-- a --> text\ntext\n");
        let invisible: Vec<bool> = (0..5).map(|i| doc.is_invisible(i)).collect();
        assert_eq!(invisible, [true, true, true, false, false]);
    }

    #[test]
    fn render_keeps_the_final_newline_state() {
        let doc = Document::parse("a\nb");
        assert_eq!(doc.render(&["x".to_owned()]), "x");
        let doc = Document::parse("a\n");
        assert_eq!(doc.render(&["x".to_owned()]), "x\n");
    }
}

#[cfg(test)]
mod timing {
    #[test]
    #[ignore = "manual profiling"]
    #[allow(clippy::print_stdout)]
    fn parse_stages() {
        use std::time::Instant;
        let path = std::env::var("README").expect("README=path");
        let text = std::fs::read_to_string(path).expect("readable");
        let options = super::markdown::options();
        let t = Instant::now();
        let arena = super::Arena::new();
        let root = comrak::parse_document(&arena, &text, &options);
        println!("comrak parse      {:?}", t.elapsed());
        let t = Instant::now();
        let mut total = 0;
        for node in root.children() {
            total += super::Collector::of(node).text.len();
        }
        println!("collect children  {:?} ({total} bytes of text)", t.elapsed());
        let t = Instant::now();
        let mut html = String::new();
        comrak::format_html(root, &options, &mut html).expect("render");
        println!("render document   {:?}", t.elapsed());
        let t = Instant::now();
        let tags = crate::html::tokenize(&html).len();
        println!("tokenize document {:?} ({tags} tags)", t.elapsed());
        let t = Instant::now();
        let raw = html5gum::Tokenizer::new(&html).flatten().count();
        println!("html5gum only     {:?} ({raw} tokens)", t.elapsed());
        let t = Instant::now();
        let _ = super::Document::parse(&text);
        println!("Document::parse   {:?}", t.elapsed());
    }
}
