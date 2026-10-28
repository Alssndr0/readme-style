"""A parsed README: blocks with line spans, headings with GitHub anchors, managed regions.

Everything readme-style generates is delimited by HTML comments, so the document itself
is the only state. Two kinds of markers exist:

* regions, a start and an end marker on their own lines, with generated content between:

      <!-- readme-style:header theme=startup -->
      ...
      <!-- /readme-style:header -->

* wrapper lines, a single generated line that ends with a marker, used to wrap user
  content without touching it, for example ``<div align="center"><!-- readme-style:center -->``.

Markers inside code blocks are ignored, so a README can document them.
"""

from __future__ import annotations

import html
import re
from dataclasses import dataclass
from functools import cached_property
from urllib.parse import urlparse

from markdown_it.token import Token

from .markdown import MD
from .slug import Slugger

_NAME = r"[a-z][a-z0-9-]*"
REGION_START = re.compile(rf"^\s*<!--\s*readme-style:({_NAME})((?:\s+{_NAME}=[\w.-]+)*)\s*-->\s*$")
REGION_END = re.compile(rf"^\s*<!--\s*/readme-style:({_NAME})\s*-->\s*$")
WRAPPER = re.compile(rf"^\s*\S.*?<!--\s*(/?)readme-style:({_NAME})\s*-->\s*$")

_IMG_TAG = re.compile(r"<img\b[^>]*>", re.I)
_A_TAG = re.compile(r"<a\b[^>]*>(.*?)</a>", re.I | re.S)
_ATTR = re.compile(r"""([a-zA-Z_:][\w:.-]*)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+))""")
_TAG = re.compile(r"<[^>]*>")
_HTML_H1 = re.compile(r"<h1[\s>]", re.I)
_TOC_TITLE = re.compile(r"\W*(table of contents|contents|toc)\W*", re.I)

BADGE_HOSTS = {
    "img.shields.io",
    "shields.io",
    "badgen.net",
    "badge.fury.io",
    "codecov.io",
    "coveralls.io",
    "travis-ci.org",
    "travis-ci.com",
    "circleci.com",
    "ci.appveyor.com",
    "api.codeclimate.com",
    "app.codacy.com",
    "snyk.io",
    "pepy.tech",
    "static.pepy.tech",
    "goreportcard.com",
    "results.pre-commit.ci",
    "api.securityscorecards.dev",
    "www.bestpractices.dev",
    "deepwiki.com",
}


def start_marker(name: str, **attrs: str) -> str:
    extra = "".join(f" {k}={v}" for k, v in attrs.items())
    return f"<!-- readme-style:{name}{extra} -->"


def end_marker(name: str) -> str:
    return f"<!-- /readme-style:{name} -->"


def wrapper_line(html_text: str, name: str, *, closing: bool = False) -> str:
    return f"{html_text}<!-- {'/' if closing else ''}readme-style:{name} -->"


def is_badge_url(url: str) -> bool:
    parsed = urlparse(url.strip())
    host = parsed.netloc.lower()
    if host in BADGE_HOSTS:
        return True
    path = parsed.path.lower()
    return "badge" in path and (path.endswith(".svg") or "/badge" in path)


def tag_attrs(tag: str) -> dict[str, str]:
    return {m[1].lower(): html.unescape(m[2] or m[3] or m[4] or "") for m in _ATTR.finditer(tag)}


def strip_tags(text: str) -> str:
    return html.unescape(_TAG.sub("", text))


def plain_text(inline: Token | None) -> str:
    """The text a browser would show for an inline token (GitHub's textContent)."""
    if inline is None or not inline.children:
        return ""
    parts = []
    for child in inline.children:
        if child.type in ("text", "code_inline"):
            parts.append(child.content)
        elif child.type in ("softbreak", "hardbreak"):
            parts.append(" ")
    return "".join(parts).strip()


@dataclass
class Region:
    name: str
    start: int
    end: int  # the end marker line, inclusive
    attrs: dict[str, str]


@dataclass
class Block:
    kind: str
    start: int
    end: int
    tokens: list[Token]

    @property
    def inline(self) -> Token | None:
        return next((t for t in self.tokens if t.type == "inline"), None)

    @property
    def content(self) -> str:
        return self.tokens[0].content


@dataclass
class Heading:
    level: int
    start: int
    end: int
    source: str
    text: str
    slug: str
    managed: bool


@dataclass
class Title:
    start: int
    end: int
    text: str
    heading: Heading | None  # None when the title is an HTML block


@dataclass
class Link:
    href: str
    text: str
    line: int


@dataclass
class Image:
    src: str
    alt: str
    block: Block


class Document:
    def __init__(self, text: str) -> None:
        self.text = text
        self.final_newline = text.endswith("\n")
        body = text[:-1] if self.final_newline else text
        self.lines: list[str] = body.split("\n") if text else []
        self.tokens = MD.parse(text)
        self.warnings: list[str] = []
        self.code_lines = self._code_lines()
        self.regions = self._find_regions()
        self.managed: set[int] = set()
        for region in self.regions:
            self.managed.update(range(region.start, region.end + 1))
        self.wrapper_lines = [
            i
            for i, line in enumerate(self.lines)
            if i not in self.code_lines and i not in self.managed and WRAPPER.match(line)
        ]
        self.managed.update(self.wrapper_lines)
        self.blocks = self._find_blocks()
        self.headings = self._find_headings()

    # parsing

    def _code_lines(self) -> set[int]:
        lines: set[int] = set()
        for tok in self.tokens:
            if tok.type in ("fence", "code_block") and tok.map:
                lines.update(range(tok.map[0], tok.map[1]))
        return lines

    def _find_regions(self) -> list[Region]:
        regions: list[Region] = []
        current: tuple[str, int, dict[str, str]] | None = None
        for i, line in enumerate(self.lines):
            if i in self.code_lines:
                continue
            start = REGION_START.match(line)
            end = REGION_END.match(line)
            if current is None:
                if start:
                    current = (start[1], i, _parse_attrs(start[2]))
                elif end:
                    self.warnings.append(f"line {i + 1}: closing marker for '{end[1]}' has no opening marker")
            elif end and end[1] == current[0]:
                regions.append(Region(current[0], current[1], i, current[2]))
                current = None
            elif start and start[1] == current[0]:
                self.warnings.append(f"line {current[1] + 1}: marker for '{current[0]}' is never closed")
                current = (start[1], i, _parse_attrs(start[2]))
        if current is not None:
            self.warnings.append(f"line {current[1] + 1}: marker for '{current[0]}' is never closed")
        return regions

    def _find_blocks(self) -> list[Block]:
        blocks: list[Block] = []
        toks = self.tokens
        i = 0
        while i < len(toks):
            tok = toks[i]
            if tok.nesting == 1:
                depth, j = 0, i
                while True:
                    depth += toks[j].nesting
                    if depth == 0:
                        break
                    j += 1
                if tok.map:
                    blocks.append(Block(tok.type.removesuffix("_open"), tok.map[0], tok.map[1], toks[i : j + 1]))
                i = j + 1
            else:
                if tok.map:
                    blocks.append(Block(tok.type, tok.map[0], tok.map[1], [tok]))
                i += 1
        return blocks

    def _find_headings(self) -> list[Heading]:
        slugger = Slugger()
        headings = []
        for i, tok in enumerate(self.tokens):
            if tok.type != "heading_open":
                continue
            inline = self.tokens[i + 1]
            text = plain_text(inline)
            slug = slugger.slug(text)
            if tok.level == 0 and tok.map:
                headings.append(
                    Heading(
                        int(tok.tag[1]), tok.map[0], tok.map[1], inline.content, text, slug, tok.map[0] in self.managed
                    )
                )
        return headings

    # output

    def render(self, lines: list[str]) -> str:
        if not lines:
            return ""
        return "\n".join(lines) + ("\n" if self.final_newline else "")

    # lookups

    def region(self, name: str) -> Region | None:
        return next((r for r in self.regions if r.name == name), None)

    def heading_at(self, line: int) -> Heading | None:
        return next((h for h in self.headings if h.start == line), None)

    @cached_property
    def user_blocks(self) -> list[Block]:
        return [b for b in self.blocks if b.start not in self.managed]

    @cached_property
    def user_text(self) -> str:
        return "\n".join(line for i, line in enumerate(self.lines) if i not in self.managed)

    @cached_property
    def title(self) -> Title | None:
        only_media = True  # nothing but logos and HTML so far
        for block in self.user_blocks:
            if block.kind == "heading":
                heading = self.heading_at(block.start)
                if heading is None:
                    return None
                # An H1, or an opening heading that no other heading outranks (a README
                # that starts with "## name" and uses ### for its sections).
                others = [h for h in self.headings if not h.managed and h is not heading]
                if heading.level == 1 or (only_media and others and all(h.level > heading.level for h in others)):
                    return Title(block.start, block.end, heading.text, heading)
                return None
            if block.kind == "html_block" and _HTML_H1.search(block.content):
                return Title(block.start, block.end, strip_tags(block.content).strip(), None)
            if block.kind not in ("paragraph", "html_block", "hr"):
                return None
            only_media = only_media and (block.kind == "html_block" or is_media_block(block))
        return None

    @cached_property
    def body_headings(self) -> list[Heading]:
        after = self.title.end if self.title else 0
        return [h for h in self.headings if not h.managed and h.start >= after]

    @cached_property
    def section_level(self) -> int | None:
        return min((h.level for h in self.body_headings), default=None)

    @cached_property
    def sections(self) -> list[Heading]:
        return [h for h in self.body_headings if h.level == self.section_level]

    @cached_property
    def intro_end(self) -> int:
        return self.sections[0].start if self.sections else len(self.lines)

    @cached_property
    def intro_blocks(self) -> list[Block]:
        after = self.title.end if self.title else 0
        return [b for b in self.user_blocks if after <= b.start < self.intro_end]

    @cached_property
    def tagline(self) -> Block | None:
        """A short first paragraph after the title, the README's one-line pitch."""
        for block in self.intro_blocks:
            if is_media_block(block):
                continue
            if block.kind == "paragraph":
                text, lines = plain_text(block.inline), block.end - block.start
            elif block.kind == "html_block" and not _IMG_TAG.search(block.content):
                text = strip_tags(block.content).strip()
                lines = len(text.splitlines())
            else:
                return None
            if text and len(text) <= 200 and lines <= 2:
                return block
            return None
        return None

    def _is_tagline(self, block: Block) -> bool:
        return self.tagline is not None and block.start == self.tagline.start

    @cached_property
    def hero_start(self) -> int:
        """First line of the hero: the title plus the logos and centered HTML right above it."""
        if not self.title:
            return self.user_blocks[0].start if self.user_blocks and self.hero_end else 0
        start = self.title.start
        for block in reversed([b for b in self.user_blocks if b.start < self.title.start]):
            if not (is_media_block(block) or _is_hero_html(block)):
                break
            start = block.start
        return start

    @cached_property
    def lead_end(self) -> int:
        """End of the logos and centered HTML that open a README without a title."""
        end = 0
        for block in self.user_blocks:
            if not (is_media_block(block) or _is_hero_html(block)):
                break
            end = block.end
        return end

    @cached_property
    def hero_end(self) -> int:
        """End of the hero: the title (or leading logos), then a tagline and badge rows."""
        if self.title:
            end, blocks = self.title.end, [b for b in self.intro_blocks]
        else:
            end, blocks = 0, self.intro_blocks
        for block in blocks:
            if self.title:
                ok = self._is_tagline(block) or is_badge_block(block) or _is_hero_html(block, centered=True)
            else:
                ok = self._is_tagline(block) or is_media_block(block) or _is_hero_html(block)
            if not ok:
                break
            end = block.end
        return end

    @cached_property
    def top_images(self) -> list[Image]:
        images = []
        for block in self.user_blocks:
            if block.start >= self.intro_end:
                break
            images.extend(block_images(block))
        return images

    @cached_property
    def has_user_badges(self) -> bool:
        return any(is_badge_url(img.src) for img in self.top_images)

    @cached_property
    def has_hero_image(self) -> bool:
        return any(not is_badge_url(img.src) for img in self.top_images)

    @cached_property
    def has_user_toc(self) -> bool:
        for heading in self.headings:
            if not heading.managed and _TOC_TITLE.fullmatch(heading.text):
                return True
        for block in self.user_blocks:
            marker = block.inline.content.strip().lower() if block.inline is not None else ""
            if block.kind == "paragraph" and marker in ("[toc]", "[[toc]]", "[[_toc_]]"):
                return True
            if block.kind in ("bullet_list", "ordered_list"):
                items = sum(1 for t in block.tokens if t.type == "list_item_open")
                anchors = sum(1 for t in block.tokens for c in t.children or [] if _is_anchor_link(c))
                if items >= 3 and anchors >= 0.6 * items:
                    return True
        return False

    @cached_property
    def links(self) -> list[Link]:
        """Every link in user content, in document order."""
        found: list[Link] = []
        for block in self.user_blocks:
            for tok in block.tokens:
                if tok.type == "html_block":
                    found.extend(Link(h, t, block.start) for h, t in _html_links(tok.content))
                children = tok.children or []
                for i, child in enumerate(children):
                    if child.type == "link_open":
                        text = []
                        for nxt in children[i + 1 :]:
                            if nxt.type == "link_close":
                                break
                            if nxt.type in ("text", "code_inline") or nxt.type == "image":
                                text.append(nxt.content)
                        found.append(Link(str(child.attrs.get("href", "")), "".join(text), block.start))
                    elif child.type == "html_inline":
                        found.extend(Link(h, t, block.start) for h, t in _html_links(child.content))
        return found


def _parse_attrs(raw: str) -> dict[str, str]:
    return dict(part.split("=", 1) for part in raw.split())


def _is_anchor_link(tok: Token) -> bool:
    return tok.type == "link_open" and str(tok.attrs.get("href", "")).startswith("#")


def _html_links(text: str) -> list[tuple[str, str]]:
    links = []
    for m in _A_TAG.finditer(text):
        href = tag_attrs(m.group(0).split(">", 1)[0]).get("href")
        if href:
            links.append((href, strip_tags(m.group(1)).strip()))
    return links


def block_images(block: Block) -> list[Image]:
    images = []
    for tok in block.tokens:
        if tok.type == "html_block":
            images.extend(_html_images(tok.content, block))
        for child in tok.children or []:
            if child.type == "image":
                images.append(Image(str(child.attrs.get("src", "")), child.content, block))
            elif child.type == "html_inline":
                images.extend(_html_images(child.content, block))
    return images


def _html_images(text: str, block: Block) -> list[Image]:
    images = []
    for m in _IMG_TAG.finditer(text):
        attrs = tag_attrs(m.group(0))
        if attrs.get("src"):
            images.append(Image(attrs["src"], attrs.get("alt", ""), block))
    return images


def _is_hero_html(block: Block, *, centered: bool = False) -> bool:
    """A short, self-contained HTML block, such as a centered subtitle or link row."""
    if block.kind != "html_block":
        return False
    content = block.content.lower()
    if content.count("<div") != content.count("</div") or len(strip_tags(content).strip()) > 300:
        return False
    return not centered or bool(re.search(r"align\s*=\s*[\"']?center|<center", content))


def is_badge_block(block: Block) -> bool:
    images = block_images(block)
    return bool(images) and is_media_block(block) and all(is_badge_url(i.src) for i in images)


def is_media_block(block: Block) -> bool:
    """A paragraph or HTML block holding only images, links around them, or comments."""
    if block.kind == "html_block":
        return not strip_tags(re.sub(r"<!--.*?-->", "", block.content, flags=re.S)).strip()
    if block.kind != "paragraph" or block.inline is None:
        return False
    children = block.inline.children or []
    if not any(c.type == "image" or (c.type == "html_inline" and _IMG_TAG.search(c.content)) for c in children):
        return False
    return not any(c.type in ("text", "code_inline") and c.content.strip() for c in children)
