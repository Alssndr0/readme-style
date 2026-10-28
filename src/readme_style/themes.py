"""Themes are plain data: which pieces to add and how to lay them out.

GitHub strips custom CSS, so a theme's visual range comes from layout (centering through
``align``), badge style, navigation, collapsible sections and callouts.
"""

from __future__ import annotations

from dataclasses import dataclass, fields, replace
from typing import Any

BADGE_STYLES = ("flat", "flat-square", "for-the-badge", "plastic", "social")
TOC_STYLES = ("none", "list", "details")
ALIGNS = ("left", "center")
HEADER_POSITIONS = ("after-title", "after-tagline")


@dataclass(frozen=True)
class Theme:
    name: str
    description: str
    align: str = "left"
    header_position: str = "after-title"
    logo: bool = False
    logo_width: int = 120
    picture_logos: bool = True
    tagline_from_metadata: bool = False
    badges: tuple[str, ...] = ("package", "ci", "license")
    badge_style: str = "flat-square"
    max_badges: int = 6
    nav: bool = False
    nav_max: int = 6
    divider: bool = False
    toc: str = "list"
    toc_depth: int = 2
    toc_min_sections: int = 4
    toc_title: str = "Contents"
    callouts: bool = True
    collapse: tuple[str, ...] = ()
    collapse_min_lines: int = 12
    collapse_label: str = "Click to expand"

    def validate(self) -> None:
        checks = [
            ("align", ALIGNS),
            ("badge_style", BADGE_STYLES),
            ("toc", TOC_STYLES),
            ("header_position", HEADER_POSITIONS),
        ]
        for attr, allowed in checks:
            if getattr(self, attr) not in allowed:
                raise ValueError(f"{attr.replace('_', '-')} must be one of {', '.join(allowed)}")

    def with_overrides(self, overrides: dict[str, Any]) -> Theme:
        if not overrides:
            return self
        types = {f.name: f.type for f in fields(self)}
        clean: dict[str, Any] = {}
        for key, value in overrides.items():
            attr = key.replace("-", "_")
            if attr not in types or attr in ("name", "description"):
                raise ValueError(f"unknown theme option '{key}'")
            kind = str(types[attr])
            if kind.startswith("tuple"):
                value = [value] if isinstance(value, str) else value
                if not isinstance(value, list) or not all(isinstance(v, str) for v in value):
                    raise ValueError(f"theme option '{key}' must be a list of strings")
                value = tuple(value)
            elif not _type_ok(kind, value):
                raise ValueError(f"theme option '{key}' must be of type {kind}")
            clean[attr] = value
        theme = replace(self, **clean)
        theme.validate()
        return theme


def _type_ok(kind: str, value: Any) -> bool:
    if kind == "bool":
        return isinstance(value, bool)
    if kind == "int":
        return isinstance(value, int) and not isinstance(value, bool)
    return isinstance(value, str)


CHANGELOG_LIKE = (
    "change ?log",
    "changes",
    "release notes",
    "release history",
    "history",
    "news",
    "updates",
    "what'?s new",
)

THEMES: dict[str, Theme] = {
    theme.name: theme
    for theme in (
        Theme(
            name="minimal",
            description="Pure Markdown, left aligned: badges under the title, a contents list, callouts.",
        ),
        Theme(
            name="startup",
            description="Centered hero with logo, tagline, bold badges and a section nav bar.",
            align="center",
            header_position="after-tagline",
            logo=True,
            tagline_from_metadata=True,
            badges=("package", "downloads", "ci", "coverage", "docs", "license", "community"),
            badge_style="for-the-badge",
            max_badges=6,
            nav=True,
            divider=True,
            toc="none",
            collapse=(*CHANGELOG_LIKE, "faq", "frequently asked questions"),
            collapse_min_lines=10,
        ),
        Theme(
            name="ml-research",
            description="Paper-style header: arXiv, Hugging Face and Colab badges, nav, collapsible news.",
            align="center",
            header_position="after-tagline",
            logo=True,
            tagline_from_metadata=True,
            badges=("research", "runtime", "license", "community"),
            badge_style="flat",
            max_badges=7,
            nav=True,
            toc="details",
            toc_title="Table of Contents",
            collapse=CHANGELOG_LIKE,
            collapse_min_lines=8,
        ),
    )
}

DEFAULT_THEME = "minimal"


def get_theme(name: str) -> Theme:
    try:
        return THEMES[name]
    except KeyError:
        raise ValueError(f"unknown theme '{name}' (available: {', '.join(THEMES)})") from None
