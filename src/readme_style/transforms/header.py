"""The header: a logo above the title, then tagline, badges, nav bar and divider."""

from __future__ import annotations

import re

from ..badges import select_badges
from ..document import _TOC_TITLE, Document, Heading
from ..editor import Editor
from ..themes import Theme
from .base import Context, sync_region
from .logos import render_logo

NAV_PRIORITY = [
    r"features?|highlights|overview|why\b.*",
    r"install(ation|ing)?|setup|getting started",
    r"quick ?start|usage|how to use|examples?|demo|tutorial",
    r"documentation|docs",
    r"benchmarks?|results|performance|evaluation",
    r"models?|checkpoints|datasets?|model zoo",
    r"citation|cite|citing",
    r"contribut\w*",
    r"faq|frequently asked questions",
    r"licen[cs]e",
]
_NAV_PRIORITY = [re.compile(rf"\W*(?:{p})\W*", re.I) for p in NAV_PRIORITY]
_DOCS_SECTION = re.compile(r"\W*(documentation|docs)\W*", re.I)


def _words(text: str) -> str:
    return " ".join(re.findall(r"\w+", text.lower()))


def metadata_tagline(doc: Document, description: str | None) -> str | None:
    """The package description as a tagline, if it is short and the README does not say it already."""
    description = (description or "").strip()
    if not description or len(description) > 160:
        return None
    words = _words(description).split()
    if " ".join(words[:6]) in _words(doc.user_text):
        return None
    return description


def _label(heading: Heading) -> str:
    return re.sub(r"([\\\[\]*])", r"\\\1", heading.text)


def nav_bar(doc: Document, ctx: Context) -> str | None:
    theme: Theme = ctx.theme
    sections = [h for h in doc.sections if not _TOC_TITLE.fullmatch(h.text)]
    docs = ctx.project.docs_url
    if any(_DOCS_SECTION.fullmatch(h.text) for h in sections):
        docs = None
    room = theme.nav_max - (1 if docs else 0)
    if len(sections) > room:
        picked = [h for h in sections if any(p.fullmatch(h.text) for p in _NAV_PRIORITY)][:room]
        picked += [h for h in sections if h not in picked][: room - len(picked)]
        sections = sorted(picked, key=lambda h: h.start)
    items = [f"**[{_label(h)}](#{h.slug})**" for h in sections]
    if docs:
        items.append(f"**[Docs]({docs})**")
    return " · ".join(items) if len(items) >= 2 else None


def header(doc: Document, ctx: Context) -> list[str] | None:
    theme, project = ctx.theme, ctx.project
    title = doc.title
    ed = Editor(doc.lines)

    logo_lines = None
    if theme.logo and project.logo and not doc.has_hero_image:
        name = project.name or (title.text if title else "")
        logo_lines = render_logo(project.logo, f"{name} logo".strip(), theme.logo_width)
    sync_region(ed, doc, "logo", logo_lines, title.start if title else 0, theme.name)

    content: list[str] = []
    tagline = metadata_tagline(doc, project.description) if title and doc.tagline is None else None
    if theme.tagline_from_metadata and tagline:
        content += [tagline, ""]
    ctx.badges = (
        []
        if doc.has_user_badges
        else select_badges(project, doc, theme.badges, theme.badge_style, theme.max_badges, ctx.exclude_badges)
    )
    if ctx.badges:
        content += [" ".join(badge.markdown() for badge in ctx.badges), ""]
    if theme.nav and (nav := nav_bar(doc, ctx)):
        content += [nav, ""]
    if theme.divider and (content or title):
        content.append("---")
    while content and not content[-1].strip():
        content.pop()

    if theme.header_position == "after-tagline":
        at = doc.hero_end
    else:
        at = title.end if title else doc.lead_end
    sync_region(ed, doc, "header", content, at, theme.name)
    return ed.apply() if ed.changed else None
