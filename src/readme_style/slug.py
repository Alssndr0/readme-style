"""GitHub heading anchors, following the github-slugger algorithm."""

from __future__ import annotations

import unicodedata

# github-slugger keeps letters, marks, decimal and letter numbers, connector
# punctuation (the underscore), hyphens and spaces, and drops everything else.
_KEEP = {"Lu", "Ll", "Lt", "Lm", "Lo", "Mn", "Mc", "Me", "Nd", "Nl", "Pc"}


def slugify(text: str) -> str:
    text = text.lower()
    kept = "".join(c for c in text if c in "- " or unicodedata.category(c) in _KEEP)
    return kept.replace(" ", "-")


class Slugger:
    """Produces unique anchors the way GitHub does for repeated headings."""

    def __init__(self) -> None:
        self._seen: dict[str, int] = {}

    def slug(self, text: str) -> str:
        base = slugify(text)
        result = base
        while result in self._seen:
            self._seen[base] += 1
            result = f"{base}-{self._seen[base]}"
        self._seen[result] = 0
        return result
