"""The content guarantee: styling may add decoration, never change what the README says.

Both versions are stripped of everything readme-style generates, rendered to HTML, and
reduced to a sequence of atoms: words, code spans and blocks (exact), link targets, image
sources with alt text, heading levels and block boundaries. The sequences must be equal.

Two presentation changes are allowed and normalized away: a GitHub alert marker such as
``[!NOTE]`` counts as the word it replaced ("Note:"), and emoji, which are not words, may
be dropped from a callout lead.
"""

from __future__ import annotations

import difflib
import re
from html.parser import HTMLParser

from .document import Document
from .markdown import MD

SEP = "|"
HEADINGS = {"h1", "h2", "h3", "h4", "h5", "h6"}
BLOCKS = HEADINGS | {
    "p",
    "li",
    "ul",
    "ol",
    "pre",
    "blockquote",
    "table",
    "thead",
    "tbody",
    "tr",
    "td",
    "th",
    "hr",
    "div",
    "details",
    "summary",
    "dl",
    "dt",
    "dd",
    "section",
    "article",
    "figure",
    "figcaption",
}
_ALERT = re.compile(r"\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\]", re.I)
_SHORTCODE = re.compile(r":[a-z0-9_+-]+:")
_WORD = re.compile(r"\w+")


class GuardError(Exception):
    """Raised when a styled README would not say the same thing as the original."""


def strip_managed(text: str) -> str:
    doc = Document(text)
    return doc.render([line for i, line in enumerate(doc.lines) if i not in doc.managed])


class _Atoms(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.atoms: list[str] = []
        self._code: list[str] | None = None
        self._code_depth = 0

    def _emit(self, atom: str) -> None:
        if atom == SEP and (not self.atoms or self.atoms[-1] == SEP):
            return
        self.atoms.append(atom)

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        values = {k: v or "" for k, v in attrs}
        if tag in BLOCKS:
            self._emit(SEP)
        if tag in HEADINGS:
            self._emit(tag)
        if tag == "a" and values.get("href"):
            self._emit(f"link {values['href']}")
        elif tag == "img":
            self._emit(f"image {values.get('src', '')} alt={values.get('alt', '')}")
        elif tag == "code":
            self._code_depth += 1
            if self._code is None:
                self._code = []

    def handle_endtag(self, tag: str) -> None:
        if tag == "code" and self._code_depth:
            self._code_depth -= 1
            if not self._code_depth and self._code is not None:
                self._emit("code " + "".join(self._code))
                self._code = None
        if tag in BLOCKS:
            self._emit(SEP)

    def handle_data(self, data: str) -> None:
        if self._code is not None:
            self._code.append(data)
            return
        data = _SHORTCODE.sub(" ", _ALERT.sub(lambda m: f" {m[1]} ", data))
        for word in _WORD.findall(data.casefold()):
            self._emit(f"word {word}")


def atoms(text: str) -> list[str]:
    parser = _Atoms()
    parser.feed(MD.render(strip_managed(text)))
    parser.close()
    found = parser.atoms
    while found and found[0] == SEP:
        found.pop(0)
    while found and found[-1] == SEP:
        found.pop()
    return found


def _show(items: list[str], limit: int = 12) -> str:
    shown = " ".join(item.split(" ", 1)[-1] if item != SEP else "/" for item in items[:limit])
    return shown + (" ..." if len(items) > limit else "")


def verify(before: str, after: str) -> None:
    a, b = atoms(before), atoms(after)
    if a == b:
        return
    matcher = difflib.SequenceMatcher(a=a, b=b, autojunk=False)
    for op, i1, i2, j1, j2 in matcher.get_opcodes():
        if op == "equal":
            continue
        context = _show(a[max(0, i1 - 8) : i1]) or "(start of document)"
        parts = []
        if i2 > i1:
            parts.append(f"drop [{_show(a[i1:i2])}]")
        if j2 > j1:
            parts.append(f"add [{_show(b[j1:j2])}]")
        raise GuardError(f"the result would {' and '.join(parts)} after: {context}")
