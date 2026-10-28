"""A table of contents with GitHub anchors, placed before the first section."""

from __future__ import annotations

import re

from ..document import _TOC_TITLE, Document, Heading
from ..editor import Editor
from ..themes import CHANGELOG_LIKE
from .base import Context, sync_region

_IMAGE = re.compile(r"!\[[^\]]*\]\([^)]*\)")
_LINK = re.compile(r"\[([^\]]*)\]\([^)]*\)")
_TAG = re.compile(r"<[^>]+>")
# Sections whose subheadings are entries (versions, dates, questions), not structure.
_NO_CHILDREN = re.compile(rf"\W*(?:{'|'.join((*CHANGELOG_LIKE, 'faq', 'frequently asked questions'))})\W*", re.I)


def entry_text(heading: Heading) -> str:
    """The heading's own Markdown, minus links and images, which cannot nest in a link."""
    text = _TAG.sub("", _LINK.sub(r"\1", _IMAGE.sub("", heading.source))).strip()
    return text or heading.text


def render_toc(doc: Document, ctx: Context) -> list[str] | None:
    theme = ctx.theme
    level = doc.section_level
    if level is None:
        return None
    entries: list[Heading] = []
    skip_children = False
    for h in doc.body_headings:
        if h.level == level:
            skip_children = bool(_NO_CHILDREN.fullmatch(h.text))
        elif skip_children:
            continue
        if level <= h.level < level + theme.toc_depth and not _TOC_TITLE.fullmatch(h.text):
            entries.append(h)
    if sum(1 for h in entries if h.level == level) < theme.toc_min_sections:
        return None
    if len(entries) > 40:
        entries = [h for h in entries if h.level == level]
    items = [f"{'  ' * (h.level - level)}- [{entry_text(h)}](#{h.slug})" for h in entries]
    if theme.toc == "details":
        return ["<details>", f"<summary><b>{theme.toc_title}</b></summary>", "", *items, "", "</details>"]
    return [f"**{theme.toc_title}**", "", *items]


def toc(doc: Document, ctx: Context) -> list[str] | None:
    content = None if ctx.theme.toc == "none" or doc.has_user_toc else render_toc(doc, ctx)
    ed = Editor(doc.lines)
    sync_region(ed, doc, "toc", content, doc.intro_end, ctx.theme.name)
    return ed.apply() if ed.changed else None
