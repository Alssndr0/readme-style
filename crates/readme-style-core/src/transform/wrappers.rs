//! Wrapper lines: generated lines around user content, rebuilt from scratch on every run.

use super::{Context, delete_lines, names};
use crate::document::Document;
use crate::edit::Editor;
use crate::markers::wrapper_line;
use crate::theme::Align;

/// Remove every wrapper line, so later steps see the README as the user wrote it.
pub(super) fn strip(doc: &Document, _: &mut Context<'_>) -> Option<Vec<String>> {
    let mut ed = Editor::new(&doc.lines);
    for &line in &doc.wrapper_lines {
        delete_lines(&mut ed, doc, line, line + 1);
    }
    ed.apply()
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
    let mut ed = Editor::new(&doc.lines);
    ed.insert(
        start,
        vec![wrapper_line(r#"<div align="center">"#, names::CENTER, false)],
    );
    ed.insert(end, vec![wrapper_line("</div>", names::CENTER, true)]);
    ed.apply()
}
