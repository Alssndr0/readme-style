//! Wrapper lines: generated lines around user content, rebuilt from scratch on every run.

use super::{Context, delete_lines, names};
use crate::document::{Block, Document};
use crate::edit::Editor;
use crate::markers::wrapper_line;
use crate::outline::{CENTERED, wrapper_change};
use crate::theme::Align;

/// Remove every wrapper line, so later steps see the README as the user wrote it.
pub(super) fn strip(doc: &Document, _: &mut Context<'_>) -> Option<Vec<String>> {
    let mut ed = Editor::new(&doc.lines);
    for &line in &doc.wrapper_lines {
        delete_lines(&mut ed, doc, line, line + 1);
    }
    ed.apply()
}

/// Adjust `start..end` so it nests with the README's own wrappers.
///
/// - A wrapper closed inside the span but opened before it: the span starts at the opening when
///   that wrapper centers anyway, and otherwise ends before the closing.
/// - A wrapper opened inside the span: the span ends where it closes, by `limit`. When it does
///   not close in time, the span shrinks to the longest part where everything opened is closed.
///
/// `None` when nothing is left to wrap.
fn nest(doc: &Document, mut start: usize, mut end: usize, limit: usize) -> Option<(usize, usize)> {
    let user: Vec<&Block> = doc.blocks.iter().filter(|b| !b.managed).collect();
    let within = |b: &Block, from: usize, to: usize| b.start >= from && b.start < to;
    'start: loop {
        let mut depth = 0;
        for block in user.iter().filter(|b| within(b, start, end)) {
            depth += wrapper_change(block);
            if depth < 0 {
                let opener = user
                    .iter()
                    .rev()
                    .find(|b| b.start < start && wrapper_change(b) > 0)
                    .filter(|b| CENTERED.is_match(&b.literal.to_ascii_lowercase()));
                match opener {
                    Some(opener) => start = opener.start,
                    None => end = block.start,
                }
                continue 'start;
            }
        }
        break;
    }
    let mut depth: i32 = user
        .iter()
        .filter(|b| within(b, start, end))
        .map(|b| wrapper_change(b))
        .sum();
    let after = end;
    for block in user.iter().filter(|b| within(b, after, limit)) {
        if depth <= 0 {
            break;
        }
        depth += wrapper_change(block);
        end = block.end;
    }
    if depth != 0 {
        let mut depth = 0;
        let mut balanced = start;
        for block in doc.blocks.iter().filter(|b| within(b, start, end)) {
            depth += if block.managed { 0 } else { wrapper_change(block) };
            if depth == 0 {
                balanced = block.end;
            }
        }
        end = balanced;
    }
    (end > start).then_some((start, end))
}

/// Center the hero (logo, title, tagline, header) in a div that Markdown still renders in.
pub(super) fn center(doc: &Document, cx: &mut Context<'_>) -> Option<Vec<String>> {
    if cx.theme.layout.align != Align::Center {
        return None;
    }
    let outline = doc.outline();
    let mut spans: Vec<(usize, usize)> = Vec::new();
    if outline.title.is_some() || outline.hero_end > 0 {
        spans.push((outline.hero_start, outline.hero_end));
    }
    for name in [names::LOGO, names::HEADER] {
        if let Some(region) = doc.region(name).filter(|r| r.start < outline.intro_end) {
            spans.push((region.start, region.end_exclusive()));
        }
    }
    let start = spans.iter().map(|s| s.0).min()?;
    let end = spans.iter().map(|s| s.1).max()?;
    let (start, end) = nest(doc, start, end, outline.intro_end)?;
    let mut ed = Editor::new(&doc.lines);
    ed.insert(
        start,
        vec![wrapper_line(r#"<div align="center">"#, names::CENTER, false)],
    );
    ed.insert(end, vec![wrapper_line("</div>", names::CENTER, true)]);
    ed.apply()
}
