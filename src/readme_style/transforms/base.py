"""Shared pieces for transforms: the run context and managed region syncing."""

from __future__ import annotations

from dataclasses import dataclass, field

from ..badges import Badge
from ..document import Document, Region, end_marker, start_marker
from ..editor import Editor
from ..project import Project
from ..themes import Theme


@dataclass
class Context:
    theme: Theme
    project: Project
    exclude_badges: tuple[str, ...] = ()
    badges: list[Badge] = field(default_factory=list)


def sync_region(ed: Editor, doc: Document, name: str, content: list[str] | None, at: int, theme: str) -> None:
    """Make region `name` hold `content` at line `at`; an empty `content` removes it.

    Regions always sit where the theme puts them, so the output depends only on the
    user's content and switching themes gives the same result as styling from scratch.
    """
    existing = [r for r in doc.regions if r.name == name]
    if content:
        block = [start_marker(name, theme=theme), *content, end_marker(name)]
        if existing and _at(doc, existing[0], at):
            first = existing.pop(0)
            ed.replace(first.start, first.end + 1, block, pad=True)
        else:
            ed.insert(at, block)
    for region in existing:
        ed.delete(region.start, region.end + 1)


def _at(doc: Document, region: Region, at: int) -> bool:
    """True when only blank lines separate the region from position `at`."""
    if at <= region.start:
        between = range(at, region.start)
    elif at > region.end:
        between = range(region.end + 1, at)
    else:
        return True
    return all(not doc.lines[i].strip() for i in between)
