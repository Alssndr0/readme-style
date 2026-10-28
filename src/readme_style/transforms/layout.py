"""Wrapper lines: generated lines that wrap user content, rebuilt from scratch on every run."""

from __future__ import annotations

from ..document import Document, wrapper_line
from ..editor import Editor
from .base import Context

CENTER_OPEN = wrapper_line('<div align="center">', "center")
CENTER_CLOSE = wrapper_line("</div>", "center", closing=True)


def strip_wrappers(doc: Document, ctx: Context) -> list[str] | None:
    """Remove every wrapper line, so later steps see the README as the user wrote it."""
    if not doc.wrapper_lines:
        return None
    ed = Editor(doc.lines)
    for line in doc.wrapper_lines:
        ed.delete(line, line + 1)
    return ed.apply()


def center(doc: Document, ctx: Context) -> list[str] | None:
    """Center the hero (logo, title, tagline, header) with a div that Markdown renders inside."""
    if ctx.theme.align != "center":
        return None
    logo, header = doc.region("logo"), doc.region("header")
    spans: list[tuple[int, int]] = []
    if doc.title or doc.hero_end:
        spans.append((doc.hero_start, doc.hero_end))
    if logo and logo.start < doc.intro_end:
        spans.append((logo.start, logo.end + 1))
    if header and header.start < doc.intro_end:
        spans.append((header.start, header.end + 1))
    if not spans:
        return None
    ed = Editor(doc.lines)
    ed.insert(min(s for s, _ in spans), [CENTER_OPEN])
    ed.insert(max(e for _, e in spans), [CENTER_CLOSE])
    return ed.apply()
