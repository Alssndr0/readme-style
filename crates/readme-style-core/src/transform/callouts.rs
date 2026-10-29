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

/// The alert kind and the text after the lead-in, when `text` opens with one.
fn lead(text: &str) -> Option<(String, String)> {
    let text = text.trim();
    let (kind, rest) = if let Some(c) = EMPHASIZED.captures(text) {
        let kind = (1..=4).find_map(|i| c.get(i))?;
        (kind.as_str(), c.get(5))
    } else {
        let c = PLAIN.captures(text)?;
        (c.get(1)?.as_str(), c.get(2))
    };
    Some((
        kind.to_ascii_uppercase(),
        rest.map_or("", |m| m.as_str()).trim().to_owned(),
    ))
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
    if rest.is_empty() {
        // The alert's text must continue on the next line, or the lead-in was its own paragraph.
        let continues = doc
            .lines
            .get(block.start + 1)
            .filter(|_| block.start + 1 < block.end)
            .and_then(|line| QUOTE.captures(line))
            .is_some_and(|c| !c[2].trim().is_empty());
        if !continues {
            return;
        }
    }
    let prefix = format!("{} ", first[1].trim_end());
    let mut lines = vec![format!("{prefix}[!{kind}]")];
    if !rest.is_empty() {
        lines.push(format!("{prefix}{rest}"));
    }
    ed.replace_exact(block.start, block.start + 1, lines);
}

fn paragraph(doc: &Document, block: &Block, ed: &mut Editor<'_>) {
    let lines = &doc.lines[block.start..block.end];
    let Some((kind, rest)) = lead(&lines[0]) else {
        return;
    };
    if rest.is_empty() && lines.len() == 1 {
        return;
    }
    let mut new = vec![format!("> [!{kind}]")];
    if !rest.is_empty() {
        new.push(format!("> {rest}"));
    }
    new.extend(lines[1..].iter().map(|line| format!("> {line}")));
    ed.replace(block.start, block.end, new);
}

#[cfg(test)]
mod tests {
    use super::lead;

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
