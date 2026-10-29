//! Turn "**Note:** ..." quotes and paragraphs into GitHub alerts (`> [!NOTE]`).

use std::sync::LazyLock;

use regex::Regex;

use super::Context;
use crate::document::{Block, BlockKind, Document};
use crate::edit::Editor;

const KINDS: &str = "note|tip|important|warning|caution";
const EMOJI: &str = r"(?:⚠️|⚠|ℹ️|ℹ|💡|❗️|❗|❕|📝|📌|🚨|:warning:|:information_source:|:bulb:|:exclamation:|:heavy_exclamation_mark:|:memo:|:pushpin:|:rotating_light:)";

/// `**Note:**`, `**Note**:`, `__Tip__`, `*Warning:*` and the like, then the rest of the line.
static EMPHASIZED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i)^(?:{EMOJI}\s*)?(?:\*\*({KINDS})\s*:?\*\*|__({KINDS})\s*:?__|\*({KINDS})\s*:?\*|_({KINDS})\s*:?_)\s*:?(\s.*)?$"
    ))
    .expect("valid regex")
});
/// `Note: ...`, where the colon is required.
static PLAIN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"(?i)^(?:{EMOJI}\s*)?({KINDS})\s*:(\s.*)?$")).expect("valid regex")
});
static QUOTE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^( {0,3}>[ \t]?)(.*)$").expect("valid regex"));

pub(super) fn apply(doc: &Document, cx: &mut Context<'_>) -> Option<Vec<String>> {
    if !cx.theme.callouts.enabled {
        return None;
    }
    let mut ed = Editor::new(&doc.lines);
    for block in doc.user_blocks() {
        match block.kind {
            BlockKind::BlockQuote => quote(doc, block, &mut ed),
            BlockKind::Paragraph => paragraph(doc, block, &mut ed),
            _ => {}
        }
    }
    ed.apply()
}

/// The alert kind and the text after the lead-in, when `text` opens with one. Trailing spaces
/// stay: two of them are a line break.
fn lead(text: &str) -> Option<(String, String)> {
    let text = text.trim_start();
    let (kind, rest) = if let Some(c) = EMPHASIZED.captures(text) {
        let kind = (1..=4).find_map(|i| c.get(i))?;
        (kind.as_str(), c.get(5))
    } else {
        let c = PLAIN.captures(text)?;
        (c.get(1)?.as_str(), c.get(2))
    };
    Some((
        kind.to_ascii_uppercase(),
        rest.map_or("", |m| m.as_str()).trim_start().to_owned(),
    ))
}

static ORDERED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d{1,9})([.)])(\s.*)?$").expect("valid regex"));
static RULE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:[-*_][ \t]*){3,}$").expect("valid regex"));
/// A setext underline, or a one-column table delimiter row (`--`, `:-:`, `-|`).
static UNDERLINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:=+|:?-+:?[ \t]*\|?)[ \t]*$").expect("valid regex"));
/// A link reference or footnote definition.
static DEFINITION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\[(?:[^\]\\]|\\.)*\]:").expect("valid regex"));

/// `text` made safe as the first line of the alert's paragraph. Moving text to the start of a
/// line can open another kind of block (`# of retries` would become a heading); a backslash
/// escape keeps the same characters on screen. `None` when no escape keeps the meaning.
///
/// The line must be safe twice: the guard's parser reads it as the start of the alert's
/// paragraph, while GitHub reads `[!NOTE]` as text and this line as its continuation, where
/// `--` would underline a heading and `:--` would start a table.
fn as_paragraph_start(text: &str) -> Option<String> {
    let text = text.trim_start();
    if text.starts_with(['<', '|'])
        || text.starts_with("```")
        || text.starts_with("~~~")
        || text.starts_with("[!")
    {
        return None;
    }
    if let Some(item) = ORDERED.captures(text) {
        return Some(format!(
            "{}\\{}{}",
            &item[1],
            &item[2],
            item.get(3).map_or("", |m| m.as_str())
        ));
    }
    if let Some(definition) = DEFINITION.find(text) {
        // `[x]\:` is no definition, and `[x]` stays a link wherever one is defined.
        let colon = definition.end() - 1;
        return Some(format!("{}\\{}", &text[..colon], &text[colon..]));
    }
    let list_marker = text.starts_with(['-', '+', '*']) && text[1..].starts_with([' ', '\t'])
        || matches!(text, "-" | "+" | "*");
    if text.starts_with(['#', '>', '=']) || list_marker || RULE.is_match(text) || UNDERLINE.is_match(text) {
        return Some(format!("\\{text}"));
    }
    Some(text.to_owned())
}

/// A lead-in alone on its line, then a line break: the alert title would replace the break, so
/// the paragraph stays as it is.
fn ends_with_line_break(line: &str) -> bool {
    line.ends_with("  ")
}

/// Blocks that always render something visible.
fn shows(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::List
            | BlockKind::Heading { .. }
            | BlockKind::Code { .. }
            | BlockKind::BlockQuote
            | BlockKind::Table
            | BlockKind::ThematicBreak
    )
}

fn quote(doc: &Document, block: &Block, ed: &mut Editor<'_>) {
    let Some(first) = QUOTE.captures(&doc.lines[block.start]) else {
        return;
    };
    let content = first[2].trim_start();
    if content.starts_with("[!") || content.starts_with('>') {
        return;
    }
    let Some((kind, rest)) = lead(content) else { return };
    if rest.is_empty() && ends_with_line_break(content) {
        return;
    }
    // The lead-in must open a paragraph: `> **Note:** a` above `> ===` is a heading.
    let inside: Vec<&str> = doc.lines[block.start..block.end]
        .iter()
        .map(|line| {
            QUOTE
                .captures(line)
                .and_then(|c| c.get(2))
                .map_or(line.as_str(), |m| m.as_str())
        })
        .collect();
    let opening = Document::parse(&inside.join("\n"));
    if opening
        .blocks
        .first()
        .is_none_or(|b| b.kind != BlockKind::Paragraph)
    {
        return;
    }
    let prefix = format!("{} ", first[1].trim_end());
    let mut lines = vec![format!("{prefix}[!{kind}]")];
    let mut end = block.start + 1;
    if rest.is_empty() {
        let Some(next) = doc
            .lines
            .get(end)
            .filter(|_| end < block.end)
            .and_then(|l| QUOTE.captures(l))
        else {
            return;
        };
        // The lead-in's paragraph either goes on to the next line, or ends there and the next
        // line opens a block of its own, which the alert keeps as it is. GitHub only shows an
        // alert with something visible in it, so that block must be one that always shows: raw
        // HTML may be a comment, and a footnote definition moves to the end.
        let lead_in = Document::parse(&format!("{content}\n{}\n", &next[2]));
        match (lead_in.blocks.first(), lead_in.blocks.get(1)) {
            (Some(first), _) if first.kind == BlockKind::Paragraph && first.end >= 2 => {}
            (Some(first), Some(second)) if first.kind == BlockKind::Paragraph && shows(second.kind) => {
                ed.replace_exact(block.start, end, lines);
                return;
            }
            _ => return,
        }
        // The next line opens the alert's text, and must still read as a paragraph.
        let Some(opening) = as_paragraph_start(&next[2]).filter(|o| !o.is_empty()) else {
            return;
        };
        lines.push(format!("{} {opening}", next[1].trim_end()));
        end += 1;
    } else {
        let Some(opening) = as_paragraph_start(&rest) else {
            return;
        };
        lines.push(format!("{prefix}{opening}"));
    }
    ed.replace_exact(block.start, end, lines);
}

fn paragraph(doc: &Document, block: &Block, ed: &mut Editor<'_>) {
    let lines = &doc.lines[block.start..block.end];
    let Some((kind, rest)) = lead(&lines[0]) else {
        return;
    };
    if rest.is_empty() && ends_with_line_break(&lines[0]) {
        return;
    }
    let (opening, continuation) = if rest.is_empty() {
        match lines.get(1) {
            Some(next) => (next.as_str(), &lines[2..]),
            None => return,
        }
    } else {
        (rest.as_str(), &lines[1..])
    };
    let Some(opening) = as_paragraph_start(opening) else {
        return;
    };
    let mut new = vec![format!("> [!{kind}]"), format!("> {opening}")];
    // Continuation lines stay in the paragraph; their indentation never mattered.
    new.extend(continuation.iter().map(|line| format!("> {}", line.trim_start())));
    ed.replace(block.start, block.end, new);
}

#[cfg(test)]
mod tests {
    use super::{as_paragraph_start, lead};

    #[test]
    fn openings_stay_paragraphs() {
        let cases = [
            ("# of retries", Some("\\# of retries")),
            ("1. Run setup first.", Some("1\\. Run setup first.")),
            ("- do not", Some("\\- do not")),
            ("> quoted", Some("\\> quoted")),
            ("***", Some("\\***")),
            ("*emphasis* stays", Some("*emphasis* stays")),
            ("2024 was good", Some("2024 was good")),
            ("--", Some("\\--")),
            (":--- ", Some("\\:--- ")),
            ("[foo]: https://example.com", Some("[foo]\\: https://example.com")),
            ("[^1]: see below", Some("[^1]\\: see below")),
            ("-|", Some("\\-|")),
            (":--- |", Some("\\:--- |")),
            ("[a link](x) here", Some("[a link](x) here")),
            ("<div>x</div> hello", None),
            ("```rust", None),
        ];
        for (text, expected) in cases {
            assert_eq!(as_paragraph_start(text).as_deref(), expected, "{text}");
        }
    }

    #[test]
    fn lead_ins() {
        let cases = [
            ("**Note:** text", Some(("NOTE", "text"))),
            ("**Warning**: careful", Some(("WARNING", "careful"))),
            ("__Tip__ use it", Some(("TIP", "use it"))),
            ("⚠️ **Caution:** hot", Some(("CAUTION", "hot"))),
            (":warning: **Important:** read", Some(("IMPORTANT", "read"))),
            ("Note: plain", Some(("NOTE", "plain"))),
            ("**Note**", Some(("NOTE", ""))),
            ("**Note:** a break  ", Some(("NOTE", "a break  "))),
            ("**Note:**   ", Some(("NOTE", ""))),
            ("**Note that** this is prose", None),
            ("Note that this is prose.", None),
            ("Notes: not a kind", None),
        ];
        for (text, expected) in cases {
            let got = lead(text);
            assert_eq!(
                got.as_ref().map(|(k, r)| (k.as_str(), r.as_str())),
                expected,
                "{text}"
            );
        }
    }
}
