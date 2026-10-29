//! The marker syntax: how generated content is delimited inside a README.
//!
//! Everything readme-style writes sits between HTML comments, which GitHub does not show,
//! so the README itself is the only state. There are two kinds:
//!
//! * **Regions**: a start and an end marker on their own lines, with generated content in
//!   between. The start marker records the theme that produced it.
//!
//!   ```text
//!   <!-- readme-style:header theme=startup -->
//!   ...generated...
//!   <!-- /readme-style:header -->
//!   ```
//!
//! * **Wrapper lines**: one generated line that ends with a marker, used to wrap user
//!   content without touching it, such as `<div align="center"><!-- readme-style:center -->`.
//!
//! Markers inside code blocks are ignored, so a README can document them.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use regex::Regex;

use crate::Diagnostic;

static REGION_START: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*<!--\s*readme-style:([a-z][a-z0-9-]*)((?:\s+[a-z][a-z0-9-]*=[\w.-]+)*)\s*-->\s*$")
        .expect("valid regex")
});
static REGION_END: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*<!--\s*/readme-style:([a-z][a-z0-9-]*)\s*-->\s*$").expect("valid regex")
});
static WRAPPER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*\S.*?<!--\s*/?readme-style:[a-z][a-z0-9-]*\s*-->\s*$").expect("valid regex")
});

/// A block of generated content delimited by a start and an end marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Region {
    pub(crate) name: String,
    /// Line of the start marker (0-based).
    pub(crate) start: usize,
    /// Line of the end marker (0-based, inclusive).
    pub(crate) end: usize,
    pub(crate) attrs: BTreeMap<String, String>,
}

impl Region {
    /// One past the end marker: the region spans `start..end_exclusive()`.
    pub(crate) fn end_exclusive(&self) -> usize {
        self.end + 1
    }
}

pub(crate) fn start_marker(name: &str, theme: &str) -> String {
    format!("<!-- readme-style:{name} theme={theme} -->")
}

pub(crate) fn end_marker(name: &str) -> String {
    format!("<!-- /readme-style:{name} -->")
}

pub(crate) fn wrapper_line(html: &str, name: &str, closing: bool) -> String {
    let slash = if closing { "/" } else { "" };
    format!("{html}<!-- {slash}readme-style:{name} -->")
}

/// Regions and wrapper lines found in a README.
#[derive(Debug, Default)]
pub(crate) struct Scan {
    pub(crate) regions: Vec<Region>,
    pub(crate) wrappers: Vec<usize>,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

/// Find markers in `lines`, skipping lines for which `in_code` is true.
pub(crate) fn scan(lines: &[String], in_code: &[bool]) -> Scan {
    let mut scan = Scan::default();
    let mut open: Option<(String, usize, BTreeMap<String, String>)> = None;
    for (i, line) in lines.iter().enumerate() {
        if in_code.get(i).copied().unwrap_or(false) {
            continue;
        }
        let start = REGION_START.captures(line);
        let end = REGION_END.captures(line);
        match (&open, start, end) {
            (None, Some(start), _) => open = Some((start[1].to_owned(), i, parse_attrs(&start[2]))),
            (Some((name, ..)), _, Some(end)) if &end[1] == name => {
                let (name, start, attrs) = open.take().expect("region is open");
                scan.regions.push(Region {
                    name,
                    start,
                    end: i,
                    attrs,
                });
            }
            // Regions do not nest: a new start marker means the open one was never closed.
            (Some((name, opened, _)), Some(start), _) => {
                scan.diagnostics.push(unclosed(name, *opened));
                open = Some((start[1].to_owned(), i, parse_attrs(&start[2])));
            }
            (_, None, Some(end)) => scan.diagnostics.push(Diagnostic {
                line: i + 1,
                message: format!("closing marker for '{}' has no opening marker", &end[1]),
            }),
            _ => {}
        }
    }
    if let Some((name, opened, _)) = open {
        scan.diagnostics.push(unclosed(&name, opened));
    }
    // A separate pass: lines after a marker that is never closed are not inside a region.
    let in_region = |i: usize| scan.regions.iter().any(|r| (r.start..=r.end).contains(&i));
    scan.wrappers = lines
        .iter()
        .enumerate()
        .filter(|&(i, line)| {
            !in_code.get(i).copied().unwrap_or(false) && !in_region(i) && WRAPPER.is_match(line)
        })
        .map(|(i, _)| i)
        .collect();
    scan
}

fn unclosed(name: &str, line: usize) -> Diagnostic {
    Diagnostic {
        line: line + 1,
        message: format!("marker for '{name}' is never closed"),
    }
}

fn parse_attrs(raw: &str) -> BTreeMap<String, String> {
    raw.split_whitespace()
        .filter_map(|pair| pair.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_owned).collect()
    }

    #[test]
    fn finds_regions_and_wrappers() {
        let text = lines(
            "<div align=\"center\"><!-- readme-style:center -->\n\n# T\n\n\
             <!-- readme-style:header theme=startup -->\nbadges\n<!-- /readme-style:header -->\n\n\
             </div><!-- /readme-style:center -->",
        );
        let found = scan(&text, &[]);
        assert_eq!(found.wrappers, [0, 8]);
        assert_eq!(found.regions.len(), 1);
        let region = &found.regions[0];
        assert_eq!((region.name.as_str(), region.start, region.end), ("header", 4, 6));
        assert_eq!(region.attrs.get("theme").map(String::as_str), Some("startup"));
    }

    #[test]
    fn skips_code_and_reports_broken_markers() {
        let text = lines(
            "<!-- readme-style:toc -->\n<!-- /readme-style:toc -->\n<!-- /readme-style:x -->\n<!-- readme-style:y -->",
        );
        let found = scan(&text, &[true, true, false, false]);
        assert!(found.regions.is_empty());
        let messages: Vec<_> = found.diagnostics.iter().map(ToString::to_string).collect();
        assert_eq!(
            messages,
            [
                "line 3: closing marker for 'x' has no opening marker",
                "line 4: marker for 'y' is never closed"
            ]
        );
    }

    #[test]
    fn wrappers_after_an_unclosed_marker_are_still_wrappers() {
        let text = lines("<!-- readme-style:toc -->\ntext\n</details><!-- /readme-style:collapse -->");
        let found = scan(&text, &[]);
        assert!(found.regions.is_empty());
        assert_eq!(found.wrappers, [2]);
    }

    #[test]
    fn an_unclosed_marker_does_not_swallow_later_regions() {
        let text = lines(
            "<!-- readme-style:toc -->\n<!-- readme-style:header theme=x -->\nbadges\n<!-- /readme-style:header -->",
        );
        let found = scan(&text, &[]);
        assert_eq!(
            found
                .regions
                .iter()
                .map(|r| (r.name.as_str(), r.start))
                .collect::<Vec<_>>(),
            [("header", 1)]
        );
        assert_eq!(found.diagnostics.len(), 1);
    }

    #[test]
    fn renders_markers() {
        assert_eq!(
            start_marker("toc", "minimal"),
            "<!-- readme-style:toc theme=minimal -->"
        );
        assert_eq!(end_marker("toc"), "<!-- /readme-style:toc -->");
        assert_eq!(
            wrapper_line("</div>", "center", true),
            "</div><!-- /readme-style:center -->"
        );
    }
}
