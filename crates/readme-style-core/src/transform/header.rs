//! The header: a logo above the title, then a tagline, badges, a nav bar and a divider.

use std::sync::LazyLock;

use regex::Regex;

use super::{Context, logos, names, sync_region};
use crate::badges::{ReadmeView, candidates, select};
use crate::document::{Document, Heading};
use crate::edit::Editor;
use crate::outline::is_toc_title;
use crate::theme::{HeaderPosition, NavStyle};

/// Sections a nav bar prefers when there are more than it has room for, most wanted first:
/// where to start and how to use it, then what the project shows, then the sections every
/// README ends with.
static NAV_TIERS: LazyLock<[Regex; 3]> = LazyLock::new(|| {
    [
        r"features?|highlights|overview|why\b.*|install(?:ation|ing)?|setup|getting started|quick ?start|usage|how to use|examples?|demo|tutorial|documentation|docs|api(?: reference)?|reference|configuration|config",
        r"benchmarks?|results|performance|evaluation|models?|checkpoints|datasets?|model zoo|citation|cite|citing|faq|frequently asked questions",
        r"contribut\w*|licen[cs]e",
    ]
    .map(|names| Regex::new(&format!(r"(?i)^\W*(?:{names})\W*$")).expect("valid regex"))
});
static DOCS_SECTION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\W*(documentation|docs)\W*$").expect("valid regex"));
static WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\w+").expect("valid regex"));

/// A nav bar is read at a glance: sections with longer titles are left out of it.
const NAV_LABEL_MAX_CHARS: usize = 30;
/// A tagline should read in one glance.
const TAGLINE_MAX_CHARS: usize = 160;
/// How many consecutive words of a description, found in the README, mean it says it already.
const TAGLINE_ECHO_WORDS: usize = 4;

fn words(text: &str) -> String {
    WORD.find_iter(&text.to_lowercase())
        .map(|m| m.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The package description as a tagline, if it is short and the README does not say it already:
/// any run of four of its words in the README counts as saying it.
fn metadata_tagline(doc: &Document, description: Option<&str>) -> Option<String> {
    let description = description?.trim();
    if description.is_empty() || description.chars().count() > TAGLINE_MAX_CHARS {
        return None;
    }
    let said = format!(" {} ", words(&doc.outline().user_text));
    let ours: Vec<String> = words(description).split(' ').map(str::to_owned).collect();
    let echoed = if ours.len() < TAGLINE_ECHO_WORDS {
        said.contains(&format!(" {} ", ours.join(" ")))
    } else {
        ours.windows(TAGLINE_ECHO_WORDS)
            .any(|run| said.contains(&format!(" {} ", run.join(" "))))
    };
    (!echoed).then(|| crate::markdown::escape_text(description))
}

fn nav_item(style: NavStyle, text: &str, target: &str) -> String {
    let link = format!("[{text}]({target})");
    match style {
        NavStyle::Bold => format!("**{link}**"),
        NavStyle::Plain => link,
        NavStyle::Kbd => format!("<kbd>{link}</kbd>"),
    }
}

fn nav_tier(title: &str) -> usize {
    NAV_TIERS
        .iter()
        .position(|tier| tier.is_match(title))
        .unwrap_or(NAV_TIERS.len())
}

fn nav_bar(doc: &Document, cx: &Context<'_>) -> Option<String> {
    let settings = &cx.theme.nav;
    let mut sections: Vec<&Heading> = doc
        .outline()
        .sections
        .iter()
        .map(|&i| &doc.headings[i])
        .filter(|h| !is_toc_title(&h.text) && h.text.chars().count() <= NAV_LABEL_MAX_CHARS)
        .collect();
    let docs = cx
        .facts
        .docs_url()
        .filter(|_| !sections.iter().any(|h| DOCS_SECTION.is_match(&h.text)));
    let room = settings.max.saturating_sub(usize::from(docs.is_some()));
    if sections.len() > room {
        // A stable sort keeps document order within a tier.
        sections.sort_by_key(|h| nav_tier(&h.text));
        sections.truncate(room);
        sections.sort_by_key(|h| h.start);
    }
    let mut items: Vec<String> = sections
        .iter()
        .map(|h| nav_item(settings.style, &h.label, &format!("#{}", h.slug)))
        .collect();
    if let Some(docs) = docs {
        items.push(nav_item(
            settings.style,
            "Docs",
            &crate::markdown::link_destination(docs),
        ));
    }
    (items.len() >= 2).then(|| items.join(&settings.separator))
}

pub(super) fn apply(doc: &Document, cx: &mut Context<'_>) -> Option<Vec<String>> {
    let theme = cx.theme;
    let outline = doc.outline();
    let title = outline.title.as_ref();
    let mut ed = Editor::new(&doc.lines);

    let logo = cx
        .facts
        .logo
        .as_ref()
        .filter(|_| theme.logo.show && !outline.has_hero_image);
    let logo_lines = logo.map_or_else(Vec::new, |logo| {
        let name = cx
            .facts
            .name
            .clone()
            .or_else(|| title.map(|t| t.text.clone()))
            .unwrap_or_default();
        logos::render(logo, format!("{name} logo").trim(), theme.logo.width)
    });
    sync_region(
        &mut ed,
        doc,
        &theme.name,
        names::LOGO,
        logo_lines,
        title.map_or(0, |t| t.start),
    );

    let mut content: Vec<String> = Vec::new();
    let tagline = (theme.layout.tagline_from_metadata && title.is_some() && outline.tagline.is_none())
        .then(|| metadata_tagline(doc, cx.facts.description.as_deref()))
        .flatten();
    if let Some(tagline) = tagline {
        content.extend([tagline, String::new()]);
    }
    cx.badges = if outline.has_user_badges {
        Vec::new()
    } else {
        let view = ReadmeView {
            links: &outline.links,
            text: &outline.user_text,
            intro_end: outline.intro_end,
        };
        select(
            candidates(cx.facts, view),
            &theme.badges.include,
            &theme.badges.exclude,
            theme.badges.max,
        )
    };
    if !cx.badges.is_empty() {
        let look = theme.badges.look();
        let row: Vec<String> = cx.badges.iter().map(|b| b.markdown(&look)).collect();
        content.extend([row.join(" "), String::new()]);
    }
    if theme.nav.show
        && !outline.has_user_nav
        && !outline.has_user_toc
        && let Some(nav) = nav_bar(doc, cx)
    {
        content.extend([nav, String::new()]);
    }
    if theme.layout.divider && (!content.is_empty() || title.is_some()) {
        content.push("---".into());
    }
    while content.last().is_some_and(|l| l.trim().is_empty()) {
        content.pop();
    }

    let at = match (theme.layout.header_position, title) {
        (HeaderPosition::AfterTagline, _) => outline.hero_end,
        (HeaderPosition::AfterTitle, Some(title)) => title.end,
        (HeaderPosition::AfterTitle, None) => outline.lead_end,
    };
    sync_region(&mut ed, doc, &theme.name, names::HEADER, content, at);
    ed.apply()
}
