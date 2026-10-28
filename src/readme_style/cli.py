"""Command line interface. Diff first: nothing is written without --write or a yes."""

from __future__ import annotations

import argparse
import difflib
import os
import sys
from pathlib import Path

from . import __version__
from .config import ConfigError
from .engine import IdempotencyError, Result, style_file, write_text
from .guard import GuardError
from .themes import THEMES

README_NAMES = ("README.md", "readme.md", "Readme.md", "README.markdown", "readme.markdown")

EXIT_OK, EXIT_CHANGES, EXIT_ERROR, EXIT_REFUSED = 0, 1, 2, 3


class Colors:
    def __init__(self, enabled: bool) -> None:
        self.enabled = enabled

    def __call__(self, text: str, code: str) -> str:
        return f"\033[{code}m{text}\033[0m" if self.enabled else text


def _use_color(choice: str, stream) -> bool:
    if choice != "auto":
        return choice == "always"
    return stream.isatty() and "NO_COLOR" not in os.environ and os.environ.get("TERM") != "dumb"


def render_diff(before: str, after: str, name: str, color: Colors) -> str:
    def lines(text: str) -> list[str]:
        out = text.splitlines(keepends=True)
        if out and not out[-1].endswith("\n"):
            out[-1] += "\n"
        return out

    rendered = []
    for line in difflib.unified_diff(lines(before), lines(after), f"{name} (current)", f"{name} (styled)"):
        if line.startswith(("+++", "---")):
            rendered.append(color(line, "1"))
        elif line.startswith("@@"):
            rendered.append(color(line, "36"))
        elif line.startswith("+"):
            rendered.append(color(line, "32"))
        elif line.startswith("-"):
            rendered.append(color(line, "31"))
        else:
            rendered.append(line)
    return "".join(rendered)


def explain(result: Result, path: Path) -> str:
    p = result.project
    rows = [
        f"readme-style {__version__} for {path}",
        f"theme        {result.theme.name} (from {result.theme_source})",
        f"config       {result.config.source if result.config and result.config.source else 'none'}",
        f"name         {p.name or '-'}",
        f"description  {p.description or '-'}",
        f"repository   {p.repo + ' (from ' + str(p.repo_source) + ')' if p.repo else '-'}",
        f"license      {p.license or '-'}{' (' + p.license_file + ')' if p.license_file else ''}",
    ]
    for package in p.packages:
        status = f"published: {package.publish_evidence}" if package.publish_evidence else "publish workflow not found"
        rows.append(f"package      {package.registry}:{package.name} ({status})")
    for workflow in p.workflows:
        rows.append(f"workflow     {workflow.file} '{workflow.name}' on {', '.join(sorted(workflow.triggers)) or '-'}")
    for tool, evidence in p.tools.items():
        rows.append(f"tool         {tool} ({evidence})")
    if p.logo:
        rows.append(f"logo         {p.logo.light}" + (f" (dark: {p.logo.dark})" if p.logo.dark else ""))
    if result.badges:
        rows.append("badges")
        rows += [f"  {b.id:<15} {b.reason}" for b in result.badges]
    else:
        rows.append("badges       none (the README already has badges, or nothing was inferred)")
    return "\n".join(rows) + "\n"


def find_readme(directory: Path) -> Path | None:
    return next((directory / n for n in README_NAMES if (directory / n).is_file()), None)


def _confirm(path: Path) -> bool:
    try:
        answer = input(f"Apply these changes to {path}? [y/N] ")
    except EOFError:
        return False
    return answer.strip().lower() in ("y", "yes")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="readme-style",
        description="Keep your README's words, upgrade its presentation. Shows a diff first; "
        "nothing is written unless you confirm or pass --write.",
    )
    parser.add_argument("paths", nargs="*", type=Path, metavar="PATH", help="README files or folders (default: .)")
    parser.add_argument("-t", "--theme", choices=sorted(THEMES), help="theme to apply (default: from config)")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("-w", "--write", action="store_true", help="write the changes without asking")
    mode.add_argument("--check", action="store_true", help="exit 1 if a README would change (for CI)")
    mode.add_argument("--diff", action="store_true", help="print the diff and exit")
    mode.add_argument("--stdout", action="store_true", help="print the styled README to stdout")
    parser.add_argument("--explain", action="store_true", help="show what was detected and why")
    parser.add_argument("-q", "--quiet", action="store_true", help="do not print diffs")
    parser.add_argument("--color", choices=("auto", "always", "never"), default="auto")
    parser.add_argument("--list-themes", action="store_true", help="list the built-in themes")
    parser.add_argument("--version", action="version", version=f"readme-style {__version__}")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    out, err = sys.stdout, sys.stderr
    color = Colors(_use_color(args.color, out))

    if args.list_themes:
        width = max(map(len, THEMES))
        for theme in THEMES.values():
            out.write(f"{color(theme.name.ljust(width), '1')}  {theme.description}\n")
        return EXIT_OK

    paths: list[Path] = []
    for raw in args.paths or [Path(".")]:
        path = find_readme(raw) if raw.is_dir() else raw
        if path is None or not path.is_file():
            err.write(f"readme-style: no README found at {raw}\n")
            return EXIT_ERROR
        paths.append(path)

    interactive = sys.stdin.isatty() and out.isatty()
    status = EXIT_OK
    for path in paths:
        try:
            result = style_file(path, theme=args.theme)
        except (ConfigError, ValueError) as exc:
            err.write(f"readme-style: {exc}\n")
            return EXIT_ERROR
        except GuardError as exc:
            err.write(f"readme-style: refusing to change {path}: {exc}\n")
            status = max(status, EXIT_REFUSED)
            continue
        except IdempotencyError as exc:
            err.write(f"readme-style: {path}: {exc}\n")
            status = max(status, EXIT_REFUSED)
            continue

        for warning in result.warnings:
            err.write(f"readme-style: {path}: {warning}\n")
        if args.explain:
            out.write(explain(result, path))
        if args.stdout:
            out.write(result.output)
            continue
        if not result.changed:
            if not args.quiet or args.check:
                out.write(f"{path}: already styled ({result.theme.name})\n")
            continue
        diff = render_diff(result.original, result.output, str(path), color)
        if args.write:
            write_text(path, result.output)
            out.write(f"{path}: restyled ({result.theme.name})\n")
        elif args.check:
            if not args.quiet:
                out.write(diff)
            out.write(f"{path}: would be restyled ({result.theme.name})\n")
            status = max(status, EXIT_CHANGES)
        else:
            if not args.quiet:
                out.write(diff)
            if args.diff:
                continue
            if interactive and _confirm(path):
                write_text(path, result.output)
                out.write(f"{path}: restyled ({result.theme.name})\n")
            elif not interactive:
                out.write(f"{path}: run with --write to apply ({result.theme.name})\n")
    return status
