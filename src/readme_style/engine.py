"""Run the pipeline, then check the content guarantee and idempotency before returning."""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

from .badges import Badge
from .config import Config, load_config
from .document import Document
from .guard import GuardError, verify
from .project import Project, load_project
from .themes import DEFAULT_THEME, Theme, get_theme
from .transforms import PIPELINE, Context

__all__ = ["GuardError", "IdempotencyError", "Result", "recorded_theme", "style_file", "style_text"]

BOM = "﻿"


class IdempotencyError(RuntimeError):
    """A second run would change the output again; that is a bug in readme-style."""


@dataclass
class Result:
    original: str
    output: str
    theme: Theme
    project: Project
    theme_source: str = "argument"
    badges: list[Badge] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)
    config: Config | None = None

    @property
    def changed(self) -> bool:
        return self.output != self.original


def run_pipeline(text: str, ctx: Context) -> str:
    for step in PIPELINE:
        doc = Document(text)
        lines = step(doc, ctx)
        if lines is not None:
            text = doc.render(lines)
    return text


def recorded_theme(text: str) -> str | None:
    """The theme named in the README's own markers, left there by a previous run."""
    for region in Document(text).regions:
        if region.attrs.get("theme"):
            return region.attrs["theme"]
    return None


def style_text(text: str, theme: Theme, project: Project, *, exclude_badges: tuple[str, ...] = ()) -> Result:
    bom = BOM if text.startswith(BOM) else ""
    newline = "\r\n" if "\r\n" in text else "\n"
    source = text.removeprefix(BOM).replace("\r\n", "\n")
    ctx = Context(theme, project, exclude_badges)
    output = run_pipeline(source, ctx)
    verify(source, output)
    if run_pipeline(output, Context(theme, project, exclude_badges)) != output:
        raise IdempotencyError("a second run would change the README again; please report this as a bug")
    return Result(
        original=text,
        output=bom + output.replace("\n", newline),
        theme=theme,
        project=project,
        badges=ctx.badges,
        warnings=Document(source).warnings,
    )


def read_text(path: Path) -> str:
    with open(path, encoding="utf-8", newline="") as handle:
        return handle.read()


def write_text(path: Path, text: str) -> None:
    with open(path, "w", encoding="utf-8", newline="") as handle:
        handle.write(text)


def style_file(path: Path, theme: str | None = None) -> Result:
    text = read_text(path)
    config = load_config(path)
    if theme:
        name, source = theme, "--theme"
    elif config.theme:
        name, source = config.theme, config.source or "config"
    elif recorded := recorded_theme(text.replace("\r\n", "\n")):
        name, source = recorded, f"markers in {path.name}"
    else:
        name, source = DEFAULT_THEME, "default"
    resolved = get_theme(name).with_overrides(config.overrides)
    project = load_project(path, repo=config.repo, logo=config.logo)
    result = style_text(text, resolved, project, exclude_badges=config.exclude_badges)
    result.theme_source = source
    result.config = config
    return result
