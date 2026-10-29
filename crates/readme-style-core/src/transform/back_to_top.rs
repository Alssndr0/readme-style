//! "Back to top" links at the end of long sections, pointing at the title's anchor.

use super::{Context, content_end, names, sync_regions};
use crate::document::Document;
use crate::edit::Editor;
use crate::html::escape;

pub(super) fn apply(doc: &Document, cx: &mut Context<'_>) -> Option<Vec<String>> {
    let settings = &cx.theme.back_to_top;
    let outline = doc.outline();
    let target = outline
        .title
        .as_ref()
        .and_then(|t| doc.heading_at(t.start))
        .map(|h| h.slug.clone());
    let mut placements = Vec::new();
    if let (true, Some(target)) = (settings.enabled, target) {
        let link = format!(
            r##"<p align="right"><a href="#{target}">{} ↑</a></p>"##,
            escape(&settings.label)
        );
        let starts: Vec<usize> = outline.sections.iter().map(|&i| doc.headings[i].start).collect();
        for (i, &start) in starts.iter().enumerate() {
            let next = starts.get(i + 1).copied().unwrap_or(doc.lines.len());
            let end = content_end(doc, start, next);
            let lines = (start..end)
                .filter(|&j| !doc.lines[j].trim().is_empty() && !doc.is_managed(j))
                .count();
            if lines >= settings.min_lines {
                placements.push((end, vec![link.clone()]));
            }
        }
    }
    let mut ed = Editor::new(&doc.lines);
    sync_regions(&mut ed, doc, &cx.theme.name, names::BACK_TO_TOP, placements);
    ed.apply()
}
