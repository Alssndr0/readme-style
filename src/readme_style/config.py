"""Configuration from .readme-style.toml or [tool.readme-style] in pyproject.toml."""

from __future__ import annotations

import re
import tomllib
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from .badges import unknown_entries
from .project import find_root
from .themes import THEMES


class ConfigError(ValueError):
    pass


@dataclass
class Config:
    source: str | None = None
    theme: str | None = None
    repo: str | None = None
    logo: str | None = None
    exclude_badges: tuple[str, ...] = ()
    overrides: dict[str, Any] = field(default_factory=dict)


def _string_list(key: str, value: Any, source: str) -> list[str]:
    if not isinstance(value, list) or not all(isinstance(v, str) for v in value):
        raise ConfigError(f"{source}: '{key}' must be a list of strings")
    return value


def parse_config(table: dict[str, Any], source: str) -> Config:
    config = Config(source=source)
    for key, value in table.items():
        if key == "theme":
            if not isinstance(value, str) or value not in THEMES:
                raise ConfigError(f"{source}: theme must be one of {', '.join(THEMES)}")
            config.theme = value
        elif key == "repo":
            if not isinstance(value, str) or not re.fullmatch(r"[\w.-]+/[\w.-]+", value):
                raise ConfigError(f"{source}: repo must look like 'owner/name'")
            config.repo = value
        elif key == "logo":
            if isinstance(value, str):
                config.logo = value
                config.overrides["logo"] = True
            elif isinstance(value, bool):
                config.overrides["logo"] = value
            else:
                raise ConfigError(f"{source}: logo must be a path or true/false")
        elif key in ("badges", "exclude-badges"):
            entries = _string_list(key, value, source)
            if unknown := unknown_entries(entries):
                raise ConfigError(f"{source}: unknown badge or group {', '.join(map(repr, unknown))}")
            if key == "badges":
                config.overrides["badges"] = entries
            else:
                config.exclude_badges = tuple(entries)
        else:
            config.overrides[key] = value
    try:
        THEMES["minimal"].with_overrides(config.overrides)
    except ValueError as exc:
        raise ConfigError(f"{source}: {exc}") from None
    return config


def load_config(readme: Path) -> Config:
    """The first config found walking up from the README's folder to the repository root."""
    here = readme.resolve().parent
    root = find_root(here)
    while True:
        dotfile = here / ".readme-style.toml"
        if dotfile.is_file():
            return parse_config(_load(dotfile), str(dotfile))
        pyproject = here / "pyproject.toml"
        if pyproject.is_file():
            table = _load(pyproject).get("tool", {}).get("readme-style")
            if table is not None:
                return parse_config(table, f"{pyproject} [tool.readme-style]")
        if here == root or here.parent == here:
            return Config()
        here = here.parent


def _load(path: Path) -> dict[str, Any]:
    try:
        return tomllib.loads(path.read_text(encoding="utf-8"))
    except tomllib.TOMLDecodeError as exc:
        raise ConfigError(f"{path}: {exc}") from None
