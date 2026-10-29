//! Markdown options shared by every module, set up like GitHub's renderer (cmark-gfm).

use comrak::Options;

/// GitHub Flavored Markdown as GitHub renders a README: tables, strikethrough, autolinks,
/// task lists, footnotes and alerts, with raw HTML passed through.
pub(crate) fn options() -> Options<'static> {
    let mut options = Options::default();
    let extension = &mut options.extension;
    extension.strikethrough = true;
    extension.table = true;
    extension.autolink = true;
    extension.tasklist = true;
    extension.footnotes = true;
    extension.alerts = true;
    options.render.r#unsafe = true;
    options
}

/// Render a whole document to HTML, with `data-sourcepos` on block elements.
pub(crate) fn to_html_with_sourcepos(text: &str) -> String {
    let mut options = options();
    options.render.sourcepos = true;
    comrak::markdown_to_html(text, &options)
}
