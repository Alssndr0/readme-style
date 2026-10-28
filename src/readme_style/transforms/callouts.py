"""Turn "**Note:** ..." quotes and paragraphs into GitHub alerts (> [!NOTE])."""

from __future__ import annotations

import re

from ..document import Block, Document
from ..editor import Editor
from .base import Context

_KINDS = "note|tip|important|warning|caution"
_EMOJI = (
    r"(?:⚠️|⚠|ℹ️|ℹ|💡|❗️|❗|❕|📝|📌|🚨|:warning:|:information_source:|:bulb:|:exclamation:"
    r"|:heavy_exclamation_mark:|:memo:|:pushpin:|:rotating_light:)"
)
LEAD = re.compile(
    rf"^(?:{_EMOJI}\s*)?"
    rf"(?:(?P<mark>\*\*|__|\*|_)(?P<bold>{_KINDS})\s*:?(?P=mark)\s*:?|(?P<plain>{_KINDS})\s*:)"
    r"(?P<rest>(?:\s.*)?)$",
    re.I,
)
_QUOTE = re.compile(r"^( {0,3}>[ \t]?)(.*)$")


def callouts(doc: Document, ctx: Context) -> list[str] | None:
    if not ctx.theme.callouts:
        return None
    ed = Editor(doc.lines)
    for block in doc.user_blocks:
        if block.kind == "blockquote":
            _quote(doc, block, ed)
        elif block.kind == "paragraph":
            _paragraph(doc, block, ed)
    return ed.apply() if ed.changed else None


def _match(text: str) -> tuple[str, str] | None:
    lead = LEAD.match(text.strip())
    if not lead:
        return None
    return (lead["bold"] or lead["plain"]).upper(), lead["rest"].strip()


def _quote(doc: Document, block: Block, ed: Editor) -> None:
    quote = _QUOTE.match(doc.lines[block.start])
    if not quote or quote[2].lstrip().startswith(("[!", ">")):
        return
    found = _match(quote[2])
    if not found:
        return
    kind, rest = found
    if not rest and block.end - block.start <= 1:
        return
    prefix = quote[1].rstrip() + " "
    ed.replace(block.start, block.start + 1, [f"{prefix}[!{kind}]"] + ([f"{prefix}{rest}"] if rest else []))


def _paragraph(doc: Document, block: Block, ed: Editor) -> None:
    lines = doc.lines[block.start : block.end]
    found = _match(lines[0])
    if not found:
        return
    kind, rest = found
    if not rest and len(lines) == 1:
        return
    new = [f"> [!{kind}]"] + ([f"> {rest}"] if rest else []) + [f"> {line}" for line in lines[1:]]
    ed.replace(block.start, block.end, new)
