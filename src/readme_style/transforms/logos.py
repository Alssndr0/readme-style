"""Logos: render the hero logo, and give an existing logo a dark-mode variant via <picture>."""

from __future__ import annotations

import html
import posixpath

from ..document import _IMG_TAG, Document, tag_attrs
from ..editor import Editor
from ..project import Logo, normalize_src
from .base import Context

DARK_SOURCE = '<source media="(prefers-color-scheme: dark)" srcset="{}">'


def _attr(value: str) -> str:
    return html.escape(value, quote=True)


def render_logo(logo: Logo, alt: str, width: int) -> list[str]:
    img = f'<img src="{_attr(logo.light)}" alt="{_attr(alt)}" width="{width}">'
    if not logo.dark:
        return [img]
    return ["<picture>", "  " + DARK_SOURCE.format(_attr(logo.dark)), f"  {img}", "</picture>"]


def _dark_for(src: str, logo: Logo) -> str:
    """The dark variant written the way the README writes the light one (keeps a ./ prefix)."""
    assert logo.dark
    if posixpath.dirname(normalize_src(src)) == posixpath.dirname(logo.dark):
        return src[: len(src) - len(posixpath.basename(src))] + posixpath.basename(logo.dark)
    return logo.dark


def picture_logos(doc: Document, ctx: Context) -> list[str] | None:
    logo = ctx.project.logo
    if not ctx.theme.picture_logos or not logo or not logo.dark:
        return None
    target = normalize_src(logo.light)
    ed = Editor(doc.lines)
    for block in doc.user_blocks:
        if block.start >= doc.intro_end:
            break
        if block.kind == "paragraph" and block.inline is not None:
            children = [
                c
                for c in block.inline.children or []
                if c.type != "softbreak" and not (c.type == "text" and not c.content.strip())
            ]
            kinds = [c.type for c in children]
            if kinds == ["image"]:
                image, href = children[0], None
            elif kinds == ["link_open", "image", "link_close"]:
                image, href = children[1], str(children[0].attrs.get("href", ""))
            else:
                continue
            src = str(image.attrs.get("src", ""))
            if normalize_src(src) != target:
                continue
            title = image.attrs.get("title")
            img = f'<img src="{_attr(src)}" alt="{_attr(image.content)}"' + (
                f' title="{_attr(str(title))}">' if title else ">"
            )
            lines = ["<picture>", "  " + DARK_SOURCE.format(_attr(_dark_for(src, logo))), f"  {img}", "</picture>"]
            if href:
                lines = [f'<a href="{_attr(href)}">', *lines, "</a>"]
            ed.replace(block.start, block.end, lines)
        elif block.kind == "html_block" and "<picture" not in block.content.lower():
            source = "\n".join(doc.lines[block.start : block.end])

            def upgrade(match) -> str:
                src = tag_attrs(match.group(0)).get("src", "")
                if normalize_src(src) != target:
                    return match.group(0)
                return f"<picture>{DARK_SOURCE.format(_attr(_dark_for(src, logo)))}{match.group(0)}</picture>"

            upgraded = _IMG_TAG.sub(upgrade, source)
            if upgraded != source:
                ed.replace(block.start, block.end, upgraded.split("\n"))
    return ed.apply() if ed.changed else None
