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

/// A link destination that cannot break out of `(...)`: spaces, parentheses, angle brackets and
/// control characters are percent-encoded.
pub(crate) fn link_destination(url: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(url.len());
    for c in url.trim().chars() {
        if c.is_control() || matches!(c, ' ' | '(' | ')' | '<' | '>') {
            let mut buffer = [0u8; 4];
            for byte in c.encode_utf8(&mut buffer).bytes() {
                let _ = write!(out, "%{byte:02X}");
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Plain text that renders as itself on one Markdown line: inline syntax is escaped, and so is
/// anything at the start that would open a block (a heading, a quote, a list, a rule).
pub(crate) fn escape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    for c in text.chars() {
        if matches!(
            c,
            '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>' | '&' | '|' | '~'
        ) {
            out.push('\\');
        }
        out.push(c);
    }
    if out.starts_with(['#', '-', '+', '=']) {
        out.insert(0, '\\');
    } else if let Some(i) = out.find(|c: char| !c.is_ascii_digit()).filter(|&i| i > 0) {
        // "1. text" would open a list; a backslash only escapes punctuation, so it goes before the dot.
        if out[i..].starts_with(['.', ')']) {
            out.insert(i, '\\');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{escape_text, link_destination};

    #[test]
    fn destinations_are_safe() {
        assert_eq!(link_destination("https://x.y/a b(c)"), "https://x.y/a%20b%28c%29");
        assert_eq!(
            link_destination("https://x.y/<script>"),
            "https://x.y/%3Cscript%3E"
        );
        assert_eq!(link_destination("LICENSE"), "LICENSE");
    }

    #[test]
    fn text_renders_as_itself() {
        assert_eq!(
            escape_text("A *fast* <b>tool</b>"),
            "A \\*fast\\* \\<b\\>tool\\</b\\>"
        );
        assert_eq!(escape_text("# not a heading"), "\\# not a heading");
        assert_eq!(escape_text("1. not a list"), "1\\. not a list");
        assert_eq!(escape_text("2024 was good"), "2024 was good");
        assert_eq!(escape_text("- not a list"), "\\- not a list");
        assert_eq!(escape_text("Plain words."), "Plain words.");
    }
}
