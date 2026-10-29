//! Reading HTML: tokenizing fragments, collecting text, links and images.

use html5gum::{HtmlString, Token, Tokenizer};

/// A start tag, end tag or text run, with entities decoded and names lowercased.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Tag {
    Start {
        name: String,
        attrs: Vec<(String, String)>,
        /// Written `<name/>`.
        self_closing: bool,
    },
    End(String),
    Text(String),
}

impl Tag {
    pub(crate) fn attr(&self, key: &str) -> Option<&str> {
        match self {
            Tag::Start { attrs, .. } => attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str()),
            _ => None,
        }
    }
}

/// Take ownership of tokenizer bytes as a `String`, copying only when they are not UTF-8.
fn owned(bytes: HtmlString) -> String {
    String::from_utf8(bytes.0).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

/// Tokenize an HTML fragment. Comments and doctypes are dropped.
pub(crate) fn tokenize(html: &str) -> Vec<Tag> {
    let mut tags = Vec::new();
    for token in Tokenizer::new(html).flatten() {
        match token {
            Token::StartTag(tag) => tags.push(Tag::Start {
                self_closing: tag.self_closing,
                name: owned(tag.name),
                attrs: tag
                    .attributes
                    .into_iter()
                    .map(|(k, v)| (owned(k), owned(v.value)))
                    .collect(),
            }),
            Token::EndTag(tag) => tags.push(Tag::End(owned(tag.name))),
            Token::String(text) => tags.push(Tag::Text(owned(text.value))),
            _ => {}
        }
    }
    tags
}

/// Whether `html` closes every `element` it opens: a block that leaves a `<div>` or a
/// `<details>` open wraps whatever follows it.
pub(crate) fn closes(html: &str, elements: &[&str]) -> bool {
    let mut depth = vec![0i32; elements.len()];
    for tag in tokenize(html) {
        let (name, step) = match &tag {
            Tag::Start {
                name,
                self_closing: false,
                ..
            } => (name, 1),
            Tag::End(name) => (name, -1),
            _ => continue,
        };
        if let Some(i) = elements.iter().position(|e| e.eq_ignore_ascii_case(name)) {
            depth[i] += step;
        }
    }
    depth.iter().all(|&d| d <= 0)
}

/// An `<img>`, as a browser would see it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImageRef {
    pub(crate) src: String,
    pub(crate) alt: String,
}

/// An `<a href>`, with its text content (image alt text included).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinkRef {
    pub(crate) href: String,
    pub(crate) text: String,
}

/// Text content, images and links of a block.
#[derive(Debug, Clone, Default)]
pub(crate) struct Contents {
    pub(crate) text: String,
    pub(crate) images: Vec<ImageRef>,
    pub(crate) links: Vec<LinkRef>,
}

/// Escape text for an HTML attribute value or text node.
pub(crate) fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrappers_are_closed_or_not() {
        let wrappers = ["div", "details"];
        for html in [
            r#"<div align="center"><img src="a.png"><h1>T</h1></div>"#,
            r#"<p align="center"><a href="x"><img src="a.png">"#,
            "<details><summary>More</summary>text</details>",
        ] {
            assert!(super::closes(html, &wrappers), "{html}");
        }
        for html in [
            "<details open>\n<summary>Contents</summary>",
            "<div>",
            "<DIV><div></div>",
        ] {
            assert!(!super::closes(html, &wrappers), "{html}");
        }
    }

    #[test]
    fn tokenizes_and_decodes() {
        let tags = tokenize(r#"<!-- c --><a HREF="x?a=1&amp;b=2">a &amp; b</a>"#);
        assert_eq!(tags[0].attr("href"), Some("x?a=1&b=2"));
        assert_eq!(tags[1], Tag::Text("a & b".into()));
        assert_eq!(tags[2], Tag::End("a".into()));
    }

    #[test]
    fn escapes() {
        assert_eq!(escape(r#"a "b" <c> & d"#), "a &quot;b&quot; &lt;c&gt; &amp; d");
    }
}
