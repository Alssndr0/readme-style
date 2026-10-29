//! Badges: the catalog, inference from project facts, and shields.io rendering.

mod catalog;
mod infer;
mod shields;

use serde::Serialize;

pub use catalog::{BadgeGroup, BadgeKind, BadgeSelector, expand};
pub(crate) use infer::{ReadmeView, candidates};
pub use shields::{BadgeStyle, Image, Look};

/// A badge readme-style decided to show, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Badge {
    pub kind: BadgeKind,
    /// Alt text of the image.
    pub alt: String,
    pub image: Image,
    /// Where clicking the badge goes.
    pub link: Option<String>,
    /// The evidence that earned the badge.
    pub reason: String,
}

impl Badge {
    /// The badge as Markdown: `[![alt](image)](link)`.
    pub fn markdown(&self, look: &Look<'_>) -> String {
        let image = format!(
            "![{}]({})",
            escape_alt(&self.alt),
            destination(&self.image.url(look))
        );
        match &self.link {
            Some(link) => format!("[{image}]({})", destination(link)),
            None => image,
        }
    }
}

fn escape_alt(text: &str) -> String {
    text.replace('\\', "\\\\").replace('[', "\\[").replace(']', "\\]")
}

fn destination(url: &str) -> String {
    crate::markdown::link_destination(url)
}

/// The badges a theme asks for, in its order, without excluded ones, at most `max`.
pub(crate) fn select(
    available: Vec<Badge>,
    include: &[BadgeSelector],
    exclude: &[BadgeSelector],
    max: usize,
) -> Vec<Badge> {
    let skip = expand(exclude);
    let mut available = available;
    let mut chosen = Vec::new();
    for kind in expand(include) {
        if skip.contains(&kind) {
            continue;
        }
        if let Some(i) = available.iter().position(|b| b.kind == kind) {
            chosen.push(available.swap_remove(i));
        }
    }
    chosen.truncate(max);
    chosen
}

#[cfg(test)]
mod tests {
    use super::*;

    fn license() -> Badge {
        Badge {
            kind: BadgeKind::License,
            alt: "License: MIT".into(),
            image: Image::Static {
                label: "license".into(),
                message: "MIT".into(),
                color: "blue",
                logo: None,
                logo_color: None,
            },
            link: Some("LICENSE".into()),
            reason: String::new(),
        }
    }

    #[test]
    fn renders_markdown() {
        let look = Look {
            style: BadgeStyle::Flat,
            label_color: None,
            color: None,
        };
        assert_eq!(
            license().markdown(&look),
            "[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat)](LICENSE)"
        );
    }

    #[test]
    fn selection_follows_the_theme_order() {
        let mut ci = license();
        ci.kind = BadgeKind::Ci;
        let chosen = select(
            vec![ci, license()],
            &[
                BadgeSelector::Group(BadgeGroup::License),
                BadgeSelector::Group(BadgeGroup::Ci),
            ],
            &[],
            5,
        );
        assert_eq!(
            chosen.iter().map(|b| b.kind).collect::<Vec<_>>(),
            [BadgeKind::License, BadgeKind::Ci]
        );
        let none = select(
            vec![license()],
            &[BadgeSelector::Kind(BadgeKind::License)],
            &[BadgeSelector::Kind(BadgeKind::License)],
            5,
        );
        assert!(none.is_empty());
    }
}
