//! Fold long sections such as a changelog into `<details>`, keeping the heading and anchor.

use super::{Context, content_end, names};
use crate::document::Document;
use crate::edit::Editor;
use crate::html::escape;
use crate::markers::wrapper_line;

fn opener(label: &str) -> String {
    wrapper_line(
        &format!("<details><summary>{}</summary>", escape(label)),
        names::COLLAPSE,
        false,
    )
}

pub(super) fn apply(doc: &Document, cx: &mut Context<'_>) -> Option<Vec<String>> {
    let settings = &cx.theme.collapse;
    let patterns = cx.theme.collapse_patterns();
    if patterns.is_empty() {
        return None;
    }
    let title_start = doc.outline().title.as_ref().map(|t| t.start);
    let headings: Vec<_> = doc
        .headings
        .iter()
        .filter(|h| !h.managed && Some(h.start) != title_start)
        .collect();
    let mut ed = Editor::new(&doc.lines);
    let mut covered = 0;
    for (i, heading) in headings.iter().enumerate() {
        if heading.start < covered || !patterns.iter().any(|p| p.is_match(&heading.text)) {
            continue;
        }
        let next = headings[i + 1..]
            .iter()
            .find(|h| h.level <= heading.level)
            .map_or(doc.lines.len(), |h| h.start);
        // Back-to-top links are placed after folding, so a stale one inside the body is moved or
        // removed later and must not decide whether or where the section folds.
        let back_to_top = |j: usize| {
            doc.regions
                .iter()
                .any(|r| r.name == names::BACK_TO_TOP && (r.start..=r.end).contains(&j))
        };
        let mut start = heading.end;
        while start < next && (doc.lines[start].trim().is_empty() || back_to_top(start)) {
            start += 1;
        }
        let end = content_end(doc, start, next);
        let body = start..end;
        let lines = body
            .clone()
            .filter(|&j| !doc.is_invisible(j) && !back_to_top(j))
            .count();
        let touches_managed = body.clone().any(|j| doc.is_managed(j) && !back_to_top(j));
        let already_folded = doc
            .lines
            .get(start)
            .is_some_and(|l| l.trim_start().to_ascii_lowercase().starts_with("<details"));
        // A fence that never closes runs to the end: a closing line after it would be code.
        let open_fence = doc.open_fence.is_some_and(|f| f < end);
        if lines < settings.min_lines || touches_managed || already_folded || open_fence {
            continue;
        }
        ed.insert(
            start,
            vec![opener(&settings.label.replace("{title}", &heading.text))],
        );
        ed.insert(end, vec![wrapper_line("</details>", names::COLLAPSE, true)]);
        covered = end;
    }
    ed.apply()
}
