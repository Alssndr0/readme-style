//! Logos: the hero logo, and dark-mode variants for logos already in the README.

use std::sync::LazyLock;

use regex::Regex;

use super::Context;
use crate::document::{BlockKind, Document};
use crate::edit::Editor;
use crate::html::{self, escape};
use crate::project::Logo;
use crate::urls::normalize_relative;

static IMG_TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<img\b[^>]*>").expect("valid regex"));

fn dark_source(path: &str) -> String {
    format!(
        r#"<source media="(prefers-color-scheme: dark)" srcset="{}">"#,
        escape(path)
    )
}

/// The hero logo, as a `<picture>` when there is a dark variant.
pub(super) fn render(logo: &Logo, alt: &str, width: u32) -> Vec<String> {
    let img = format!(
        r#"<img src="{}" alt="{}" width="{width}">"#,
        escape(&logo.light),
        escape(alt)
    );
    match &logo.dark {
        None => vec![img],
        Some(dark) => vec![
            "<picture>".into(),
            format!("  {}", dark_source(dark)),
            format!("  {img}"),
            "</picture>".into(),
        ],
    }
}

/// The dark variant, written the way the README writes the light one (keeping a `./`).
fn dark_for(src: &str, dark: &str) -> String {
    let dir = |path: &str| path.rsplit_once('/').map_or("", |(d, _)| d).to_owned();
    if dir(&normalize_relative(src)) == dir(dark) {
        let prefix = src.rsplit_once('/').map_or("", |(d, _)| d);
        let name = dark.rsplit('/').next().unwrap_or(dark);
        return if prefix.is_empty() {
            name.to_owned()
        } else {
            format!("{prefix}/{name}")
        };
    }
    dark.to_owned()
}

/// Give a logo the README already shows a dark-mode variant through `<picture>`.
pub(super) fn dark_variants(doc: &Document, cx: &mut Context<'_>) -> Option<Vec<String>> {
    let logo = cx.facts.logo.as_ref()?;
    let dark = logo.dark.as_deref()?;
    if !cx.theme.logo.dark_variants {
        return None;
    }
    let target = normalize_relative(&logo.light);
    let intro_end = doc.outline().intro_end;
    let mut ed = Editor::new(&doc.lines);
    for block in doc.user_blocks().take_while(|b| b.start < intro_end) {
        match block.kind {
            BlockKind::Paragraph => {
                let Some(image) = block
                    .sole_image
                    .as_ref()
                    .filter(|i| normalize_relative(&i.src) == target)
                else {
                    continue;
                };
                // A paragraph's lines can start with link reference definitions, which the
                // replacement would delete. Only replace when the image opens the block.
                let first = doc.lines[block.start].trim_start();
                if !(first.starts_with("![") || first.starts_with("[![")) {
                    continue;
                }
                let title = image
                    .title
                    .as_ref()
                    .map(|t| format!(r#" title="{}""#, escape(t)))
                    .unwrap_or_default();
                let img = format!(
                    r#"<img src="{}" alt="{}"{title}>"#,
                    escape(&image.src),
                    escape(&image.alt)
                );
                let mut lines = vec![
                    "<picture>".to_owned(),
                    format!("  {}", dark_source(&dark_for(&image.src, dark))),
                    format!("  {img}"),
                    "</picture>".to_owned(),
                ];
                if let Some(href) = &image.href {
                    lines.insert(0, format!(r#"<a href="{}">"#, escape(href)));
                    lines.push("</a>".into());
                }
                // Still a paragraph, so it stacks with its neighbours as before.
                lines.insert(0, "<p>".into());
                lines.push("</p>".into());
                // Padded: an HTML block swallows following lines until a blank one.
                ed.replace(block.start, block.end, lines);
            }
            BlockKind::Html if !block.literal.to_ascii_lowercase().contains("<picture") => {
                let source = doc.lines[block.start..block.end].join("\n");
                let upgraded = IMG_TAG.replace_all(&source, |c: &regex::Captures<'_>| {
                    let tag = &c[0];
                    let src = html::tokenize(tag)
                        .first()
                        .and_then(|t| t.attr("src").map(str::to_owned))
                        .unwrap_or_default();
                    if normalize_relative(&src) == target {
                        format!("<picture>{}{tag}</picture>", dark_source(&dark_for(&src, dark)))
                    } else {
                        tag.to_owned()
                    }
                });
                if upgraded != source {
                    ed.replace_exact(
                        block.start,
                        block.end,
                        upgraded.split('\n').map(str::to_owned).collect(),
                    );
                }
            }
            _ => {}
        }
    }
    ed.apply()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_paths_follow_the_readme_spelling() {
        assert_eq!(
            dark_for("./assets/logo.png", "assets/logo-dark.png"),
            "./assets/logo-dark.png"
        );
        assert_eq!(dark_for("logo.svg", "logo-dark.svg"), "logo-dark.svg");
        assert_eq!(
            dark_for("assets/logo.svg", "brand/logo-dark.svg"),
            "brand/logo-dark.svg"
        );
    }

    #[test]
    fn renders_picture_or_img() {
        let logo = Logo {
            light: "a.svg".into(),
            dark: None,
        };
        assert_eq!(
            render(&logo, "x logo", 120),
            [r#"<img src="a.svg" alt="x logo" width="120">"#]
        );
        let logo = Logo {
            light: "a.svg".into(),
            dark: Some("a-dark.svg".into()),
        };
        assert_eq!(render(&logo, "x", 80).len(), 4);
    }
}
