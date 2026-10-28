"""Fold long sections such as a changelog into <details>, keeping the heading and its anchor."""

from __future__ import annotations

import html
import re

from ..document import Document, wrapper_line
from ..editor import Editor
from .base import Context

CLOSE = wrapper_line("</details>", "collapse", closing=True)


def opener(label: str) -> str:
    return wrapper_line(f"<details><summary>{html.escape(label)}</summary>", "collapse")


def collapse(doc: Document, ctx: Context) -> list[str] | None:
    theme = ctx.theme
    if not theme.collapse:
        return None
    patterns = [re.compile(rf"\W*(?:{p})\W*", re.I) for p in theme.collapse]
    headings = [h for h in doc.headings if not h.managed and not (doc.title and h.start == doc.title.start)]
    ed = Editor(doc.lines)
    covered = -1
    for i, heading in enumerate(headings):
        if heading.start < covered or not any(p.fullmatch(heading.text) for p in patterns):
            continue
        following = next((h for h in headings[i + 1 :] if h.level <= heading.level), None)
        start, end = heading.end, following.start if following else len(doc.lines)
        while start < end and not doc.lines[start].strip():
            start += 1
        while end > start and not doc.lines[end - 1].strip():
            end -= 1
        body = range(start, end)
        if sum(1 for j in body if doc.lines[j].strip()) < theme.collapse_min_lines:
            continue
        if any(j in doc.managed for j in body) or doc.lines[start].lstrip().lower().startswith("<details"):
            continue
        ed.insert(start, [opener(theme.collapse_label.format(title=heading.text))])
        ed.insert(end, [CLOSE])
        covered = end
    return ed.apply() if ed.changed else None
