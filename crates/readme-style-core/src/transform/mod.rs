//! The transform pipeline. Each step reads a freshly parsed document and returns new lines,
//! or `None` when it has nothing to change.
//!
//! Order matters. Wrapper lines are stripped first, so every later step sees the README as the
//! user wrote it. User content is upgraded next (callouts, logos). Then regions move to where
//! the theme puts them, before sections fold, so a region left by another theme can never
//! decide whether a section folds. Back-to-top links follow the folds, and centering wraps the
//! final hero last.

mod back_to_top;
mod callouts;
mod collapse;
mod header;
mod logos;
mod toc;
mod wrappers;

use crate::badges::Badge;
use crate::document::Document;
use crate::edit::{Editor, SEPARATOR};
use crate::markers::{Region, end_marker, start_marker};
use crate::project::ProjectFacts;
use crate::theme::Theme;

/// Region and wrapper names. Changing one breaks READMEs styled by earlier versions.
pub(crate) mod names {
    pub(crate) const LOGO: &str = "logo";
    pub(crate) const HEADER: &str = "header";
    pub(crate) const TOC: &str = "toc";
    pub(crate) const BACK_TO_TOP: &str = "back-to-top";
    pub(crate) const CENTER: &str = "center";
    pub(crate) const COLLAPSE: &str = "collapse";
}

/// What a run knows and collects.
#[derive(Debug)]
pub(crate) struct Context<'a> {
    pub(crate) theme: &'a Theme,
    pub(crate) facts: &'a ProjectFacts,
    /// The badges the header step chose, for reporting.
    pub(crate) badges: Vec<Badge>,
}

pub(crate) type Step = fn(&Document, &mut Context<'_>) -> Option<Vec<String>>;

pub(crate) const PIPELINE: &[(&str, Step)] = &[
    ("strip-wrappers", wrappers::strip),
    ("callouts", callouts::apply),
    ("logo-variants", logos::dark_variants),
    ("header", header::apply),
    ("toc", toc::apply),
    ("collapse", collapse::apply),
    ("back-to-top", back_to_top::apply),
    ("center", wrappers::center),
];

/// Make region `name` hold `content` at line `at`; empty `content` removes it.
///
/// Regions always sit where the theme puts them, so the output depends only on the user's
/// content, and switching themes gives the same result as styling from scratch.
pub(crate) fn sync_region(
    ed: &mut Editor<'_>,
    doc: &Document,
    theme: &str,
    name: &str,
    content: Vec<String>,
    at: usize,
) {
    let mut existing: Vec<&Region> = doc.regions.iter().filter(|r| r.name == name).collect();
    if !content.is_empty() {
        let block = region_block(name, theme, content);
        match existing.iter().position(|r| next_to(doc, r, at)) {
            Some(i) => {
                let region = existing.remove(i);
                ed.replace(region.start, region.end_exclusive(), block);
            }
            None => ed.insert(at, block),
        }
    }
    for region in existing {
        remove(ed, doc, region);
    }
}

/// Delete a region without changing the structure around it.
fn remove(ed: &mut Editor<'_>, doc: &Document, region: &Region) {
    delete_lines(ed, doc, region.start, region.end_exclusive());
}

/// Delete generated lines `start..end`. Inside a block that swallowed them, the block stays
/// whole; between blocks, the blocks stay apart, with an empty comment where a blank line
/// would not keep them so.
pub(crate) fn delete_lines(ed: &mut Editor<'_>, doc: &Document, start: usize, end: usize) {
    if doc.inside_block(start, end - 1) {
        ed.delete_within(start, end);
    } else if doc.needs_separator(start, end) {
        ed.replace(start, end, vec![SEPARATOR.to_owned()]);
    } else {
        ed.delete(start, end);
    }
}

/// Like [`sync_region`] for a region that appears several times (one per placement).
pub(crate) fn sync_regions(
    ed: &mut Editor<'_>,
    doc: &Document,
    theme: &str,
    name: &str,
    placements: Vec<(usize, Vec<String>)>,
) {
    let mut existing: Vec<&Region> = doc.regions.iter().filter(|r| r.name == name).collect();
    for (at, content) in placements {
        let block = region_block(name, theme, content);
        match existing.iter().position(|r| next_to(doc, r, at)) {
            Some(i) => {
                let region = existing.remove(i);
                ed.replace(region.start, region.end_exclusive(), block);
            }
            None => ed.insert(at, block),
        }
    }
    for region in existing {
        remove(ed, doc, region);
    }
}

fn region_block(name: &str, theme: &str, content: Vec<String>) -> Vec<String> {
    let mut block = Vec::with_capacity(content.len() + 2);
    block.push(start_marker(name, theme));
    block.extend(content);
    block.push(end_marker(name));
    block
}

/// True when only blank lines and other generated lines separate the region from `at`.
fn next_to(doc: &Document, region: &Region, at: usize) -> bool {
    let between = if at <= region.start {
        at..region.start
    } else if at > region.end {
        region.end_exclusive()..at
    } else {
        return true;
    };
    between
        .into_iter()
        .all(|i| doc.lines[i].trim().is_empty() || doc.is_managed(i))
}

/// Where a section's content ends: before `next` (the next section's first line), stepping
/// back over blank lines and back-to-top links, so folded sections and back-to-top links
/// agree on the same boundary.
pub(crate) fn content_end(doc: &Document, start: usize, next: usize) -> usize {
    let in_back_to_top = |line: usize| {
        doc.regions
            .iter()
            .any(|r| r.name == names::BACK_TO_TOP && (r.start..=r.end).contains(&line))
    };
    let mut end = next;
    while end > start && (doc.lines[end - 1].trim().is_empty() || in_back_to_top(end - 1)) {
        end -= 1;
    }
    end
}
